//! Ask About Patterns (Help and Pattern menus): a window where the user asks Claude about sewing
//! patterns or about PatternFoundry, paid with Foundry credits (the Foundry Account).
//! Unless the user turns it off, a summary of the pattern open now goes with each question
//! ([`pattern_summary`]: its pieces and sizes, and the names, never the values, of the
//! measurements), and Claude can look at the pattern and change it through PatternFoundry's
//! commands ([`ask::tools`]): commands that only read run at once ([`access`]); changes, and
//! reading the measurements, wait for the user's Allow ([`decide`]); files, preferences, plug-ins
//! and windows are out of its reach. Each request goes out through the host's `http_post` service
//! ([`super::post_ai`]); [`poll`] picks the reply up and runs the round of commands it asks for.
//!
//! Adapted from StitchFoundry's Ask About Embroidery (`crates/ui-egui/src/embroidery/ask.rs`).

use std::sync::mpsc::{Receiver, TryRecvError};

use egui::RichText;
use pf_ai::AiError;
use pf_ai::ask::{self, Reply, ToolCall, ToolResult};
use pf_ai::cloud;
use serde_json::{Value, json};
use vectorcraft_engine::CommandSpec;

use crate::VectorcraftApp;
use crate::theme::Tokens;

/// The most objects the summary lists.
const MAX_LISTED: usize = 40;
/// The most commands one search returns.
const MAX_FOUND: usize = 20;
/// Millimetres in a point.
const MM_PER_PT: f64 = pf_core::MM_PER_PT;

/// What the window shows of the conversation.
#[derive(Clone, Debug, PartialEq)]
pub enum Entry {
    Question(String),
    Answer(String),
    /// A change to the pattern, made or declined, or a command that couldn't run.
    Note(String),
}

/// What a conversation is: questions, or a muslin check and the questions after it (Check My
/// Muslin: [`super::muslin`], its own instructions).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Mode {
    #[default]
    Ask,
    Muslin,
}

/// Ask About Patterns between frames. Not saved: a conversation ends with the app.
pub struct AskState {
    pub open: bool,
    /// Questions, or a muslin check and the questions after it.
    pub mode: Mode,
    /// The question being typed.
    pub input: String,
    /// The conversation as shown.
    pub entries: Vec<Entry>,
    /// The conversation as the API's messages, only ever added to (but for a question that got
    /// no answer, taken back whole).
    messages: Vec<Value>,
    /// Send a summary of the pattern open now with each question.
    pub include_pattern: bool,
    /// The question being answered.
    current: Option<Current>,
    /// The request on its way.
    pending: Option<Pending>,
    /// Changes waiting for the user's Allow.
    pub approval: Option<Approval>,
    /// Why the last question got no answer.
    pub error: Option<String>,
}

impl Default for AskState {
    fn default() -> Self {
        Self {
            open: false,
            mode: Mode::Ask,
            input: String::new(),
            entries: Vec::new(),
            messages: Vec::new(),
            include_pattern: true,
            current: None,
            pending: None,
            approval: None,
            error: None,
        }
    }
}

struct Current {
    question: String,
    /// The conversation's length before the question (taken back to it when no answer comes).
    start: usize,
    /// The requests sent for it so far.
    rounds: usize,
}

struct Pending {
    rx: Receiver<Result<Reply, AiError>>,
    /// When it was sent (egui time, seconds).
    since: f64,
}

/// A round of tool calls waiting for the user: the changes to allow, and the results of the
/// calls that already ran.
pub struct Approval {
    /// The changes asked for: (call, command id, params, what the window shows).
    pub changes: Vec<(String, String, Value, String)>,
    /// The results so far, in the order of the reply's calls; `None` for the changes.
    results: Vec<(String, Option<Result<String, String>>)>,
}

impl AskState {
    /// Claude's last answer in the conversation.
    pub fn last_answer(&self) -> Option<&str> {
        self.entries.iter().rev().find_map(|e| if let Entry::Answer(a) = e { Some(a.as_str()) } else { None })
    }

    /// The conversation as text (questions, answers and notes), for a report.
    pub fn transcript(&self) -> String {
        let lines: Vec<String> = self
            .entries
            .iter()
            .map(|e| match e {
                Entry::Question(q) => format!("Me: {q}"),
                Entry::Answer(a) => format!("Claude: {a}"),
                Entry::Note(n) => format!("({n})"),
            })
            .collect();
        lines.join("\n\n")
    }

    /// Whether a question is being answered (sent, or waiting for the user's Allow).
    pub fn asking(&self) -> bool {
        self.current.is_some()
    }

    /// Seconds since the request on its way was sent.
    pub fn waiting_for(&self, now: f64) -> Option<f64> {
        self.pending.as_ref().map(|p| (now - p.since).max(0.0))
    }
}

/// What Claude may do with a command.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Access {
    /// It only reads: it runs at once.
    Read,
    /// It changes the pattern: it waits for the user's Allow.
    Change,
    /// It reads the user's body measurements: it waits for the user's Allow too.
    Personal,
    /// Out of reach: files, preferences, plug-ins, windows and the like.
    Blocked,
}

/// Command families Claude can't run: they reach outside the pattern (files, preferences,
/// plug-ins, libraries, the clipboard) or the app's windows.
const BLOCKED: &[&str] = &[
    "file.",
    "export.",
    "print.",
    "prefs.",
    "plugin.",
    "script.",
    "links.",
    "library.",
    "clipboard.",
    "pdf.",
    "webExport.",
    "flattener.",
    "shortcutSet.",
    "dataset.",
    "help.",
    "ui.",
    "app.",
    "window.",
    "command.",
    "document.export",
    "document.save",
    "document.serialize",
    "document.json",
    "text.addFontFiles",
    "text.findFontFiles",
];

/// Commands that read the user's body measurements: personal data, read only with the user's
/// Allow (CLAUDE.md).
const PERSONAL: &[&str] = &["measure.info"];

/// Endings of commands that only read.
const READS: &[&str] =
    &[".info", ".list", ".get", ".check", ".node", ".find", ".inspect", ".current", ".length", ".measurements", ".count", ".formats"];

/// What Claude may do with `spec`.
pub fn access(spec: &CommandSpec) -> Access {
    let id = spec.id;
    if blocked(id) {
        return Access::Blocked;
    }
    if PERSONAL.contains(&id) {
        return Access::Personal;
    }
    // selecting, and queries (not journalled) that read; a query that sets something still waits
    // for the user
    let reads = id.starts_with("select.") || (!spec.journal && READS.iter().any(|e| id.ends_with(e)));
    if reads { Access::Read } else { Access::Change }
}

/// Whether command `id` is out of Claude's reach (whether or not the engine has it: the file
/// commands of the menus are the app's own).
pub fn blocked(id: &str) -> bool {
    let lower = id.to_ascii_lowercase();
    let reaches_out = ["export", ".save", ".import", ".delete", ".install", "folder", "presets"].iter().any(|w| lower.contains(w));
    reaches_out || BLOCKED.iter().any(|p| id.starts_with(p))
}

/// A command's failure as Claude is told it (in English: it isn't shown to the user).
fn to_claude(why: String) -> Result<String, String> {
    Err(why)
}

/// Whether `params` name a file or folder (never run from here, whatever the command).
fn names_a_file(params: &Value) -> bool {
    params.as_object().is_some_and(|o| o.keys().any(|k| matches!(k.as_str(), "path" | "paths" | "folder" | "file" | "files" | "dir" | "url")))
}

/// Open the window (`ui.askAboutPatterns`); with `{question}`, ask it at once.
pub fn open(app: &mut VectorcraftApp, p: &Value) -> Result<Value, String> {
    if !super::available(app) {
        return Err("Ask About Patterns needs a build that can reach the AI service".into());
    }
    app.ai.ask.open = true;
    let mut asking = false;
    if let Some(q) = p.get("question").and_then(Value::as_str).filter(|q| !q.trim().is_empty()) {
        app.ai.ask.input = q.to_string();
        send(app)?;
        asking = true;
    }
    Ok(json!({ "open": true, "asking": asking }))
}

/// A plain-text summary of the pattern open now, for the model: its pattern pieces (name, cut,
/// size, allowance), the blocks drafted, the size chart's and the size range's sizes, the
/// measurement set's name and the names of the measurements it gives (never their values: those
/// are read with measure.info, with the user's Allow), and the selected objects. `None` without a
/// document.
pub fn pattern_summary(app: &mut VectorcraftApp) -> Option<String> {
    app.session.active()?;
    let pieces = app.session.execute("piece.list", &json!({})).ok().and_then(|v| v["pieces"].as_array().cloned()).unwrap_or_default();
    let st = app.session.active()?;
    let doc = &st.doc;
    let mut lines = Vec::new();
    let ab = doc.artboards.first().map(|a| a.rect);
    if let Some(r) = ab {
        lines.push(format!("Artboard {:.0} × {:.0} mm.", r.width() * MM_PER_PT, r.height() * MM_PER_PT));
    }
    if pieces.is_empty() {
        lines.push("No pattern pieces yet.".to_string());
    } else {
        lines.push(format!("{} pattern pieces:", pieces.len()));
        for p in pieces.iter().take(MAX_LISTED) {
            let Some(id) = p["id"].as_u64() else { continue };
            let n = doc.node(vectorcraft_doc::NodeId(id));
            let size = n
                .and_then(|n| n.geometric_bounds())
                .map(|b| format!(", {:.0} × {:.0} mm", b.width() * MM_PER_PT, b.height() * MM_PER_PT))
                .unwrap_or_default();
            // An unnamed piece goes by its seam line's name (a block part's).
            let seam = n.and_then(|n| n.children()).and_then(|c| {
                c.iter()
                    .find(|c| matches!(c.extras().pattern.as_deref(), Some(pf_core::PatternTag::Mark(pf_core::Mark::Seam))))
                    .and_then(|c| c.name.clone())
            });
            let name = p["name"].as_str().filter(|s| !s.trim().is_empty()).map(str::to_string).or(seam).unwrap_or_else(|| "Pattern piece".into());
            let cut = p["cut"].as_u64().unwrap_or(1);
            let mirror = if p["mirror"].as_bool() == Some(true) { " in mirror pairs" } else { "" };
            let fabric = p["fabric"].as_str().filter(|f| !f.is_empty()).map(|f| format!(", {f}")).unwrap_or_default();
            lines.push(format!("- #{id} {name}: cut {cut}{mirror}{fabric}{size}"));
        }
    }
    let mut blocks = std::collections::BTreeSet::new();
    doc.walk(|n| {
        if let Some(b) = n.extras().block.as_deref() {
            blocks.insert(format!("{:?}", b.kind).to_lowercase());
        }
    });
    if !blocks.is_empty() {
        lines.push(format!("Blocks drafted from the measurements: {}.", blocks.into_iter().collect::<Vec<_>>().join(", ")));
    }
    if let Some(m) = doc.measurements.as_deref() {
        let names: Vec<&str> = m.values.keys().map(String::as_str).collect();
        let mut line = String::from("Measurements");
        if !m.name.trim().is_empty() {
            line.push_str(&format!(" \"{}\"", m.name.trim()));
        }
        if let Some(size) = &m.size {
            line.push_str(&format!(" (size {size} of the size chart)"));
        }
        if names.is_empty() {
            line.push_str(": none given yet.");
        } else {
            line.push_str(&format!(" given: {}.", names.join(", ")));
        }
        if !m.formulas.is_empty() {
            line.push_str(&format!(" Formulas: {}.", m.formulas.keys().map(String::as_str).collect::<Vec<_>>().join(", ")));
        }
        lines.push(line);
    } else {
        lines.push("No measurements set yet.".to_string());
    }
    if let Some(c) = doc.size_chart.as_deref() {
        lines.push(format!(
            "Size chart{}: sizes {}.",
            if c.name.is_empty() { String::new() } else { format!(" \"{}\"", c.name) },
            c.size_names().join(", ")
        ));
    }
    if let Some(r) = doc.grade_sizes.as_deref() {
        lines.push(format!("Graded into sizes {} (base {}).", r.sizes.join(", "), r.base));
    }
    let selected: Vec<String> =
        st.selection.objects.iter().take(MAX_LISTED).filter_map(|id| doc.node(*id).map(|n| format!("#{} {}", id.0, n.display_name()))).collect();
    if !selected.is_empty() {
        lines.push(format!("Selected: {}.", selected.join(", ")));
    }
    Some(lines.join("\n"))
}

/// The commands matching `query`'s words, best first, as find_commands returns them.
pub fn find(app: &VectorcraftApp, query: &str) -> Value {
    let words: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();
    let mut found: Vec<(usize, &CommandSpec)> = vectorcraft_engine::command_specs()
        .iter()
        .filter(|c| !vectorcraft_engine::cmd::is_alias(c.id) && access(c) != Access::Blocked)
        .filter_map(|c| {
            let hay = format!("{} {} {}", c.id, c.label, c.menu.join(" ")).to_lowercase();
            let score = words.iter().filter(|w| hay.contains(w.as_str())).count();
            (score > 0).then_some((score, c))
        })
        .collect();
    // most words matched first, in the registry's order otherwise
    found.sort_by_key(|f| std::cmp::Reverse(f.0));
    let list: Vec<Value> = found
        .into_iter()
        .take(MAX_FOUND)
        .map(|(_, c)| {
            let info = c.info(&app.session);
            json!({
                "id": c.id,
                "label": c.label,
                "menu": c.menu.join(" › "),
                "canRunNow": info.enabled,
                "whyNot": info.disabled_reason,
                "changesThePattern": access(c) == Access::Change,
                "readsMeasurements": access(c) == Access::Personal,
                "params": c.params,
            })
        })
        .collect();
    json!(list)
}

/// How a command shows in the window: its label and menu.
fn describe(spec: &CommandSpec) -> String {
    let menu: Vec<&str> = spec.menu.iter().copied().filter(|m| !m.is_empty()).collect();
    if menu.is_empty() { spec.label.to_string() } else { format!("{} › {}", menu.join(" › "), spec.label) }
}

/// Send the question typed (Ask). It stays in the box until the answer comes.
pub fn send(app: &mut VectorcraftApp) -> Result<(), String> {
    if app.ai.ask.asking() {
        return Err("already asking: wait for the answer".into());
    }
    let fail = |app: &mut VectorcraftApp, e: AiError| {
        let m = e.to_string();
        app.ai.ask.error = Some(m.clone());
        Err(m)
    };
    let asked = app.ai.ask.entries.iter().filter(|e| matches!(e, Entry::Question(_))).count();
    if asked >= ask::MAX_QUESTIONS {
        let m = "This conversation is long: start a New Conversation.".to_string();
        app.ai.ask.error = Some(m.clone());
        return Err(m);
    }
    super::account::key_to_send(app); // (reads the account once)
    if let Err(m) = super::account::require_account(app) {
        super::account::show_if_needed(app);
        app.ai.ask.error = Some(m.clone());
        return Err(m);
    }
    let pattern = if app.ai.ask.include_pattern { pattern_summary(app) } else { None };
    let message = match ask::question(&app.ai.ask.input, pattern.as_deref()) {
        Ok(m) => m,
        Err(e) => return fail(app, e),
    };
    let st = &mut app.ai.ask;
    let question = st.input.trim().to_string();
    st.current = Some(Current { question: question.clone(), start: st.messages.len(), rounds: 0 });
    st.messages.push(message);
    st.entries.push(Entry::Question(question));
    st.error = None;
    request(app)
}

/// Ask `question` in the conversation, shown in it as `shown` (the words typed in the box stay
/// there).
pub fn ask_as(app: &mut VectorcraftApp, question: &str, shown: &str) -> Result<(), String> {
    let typed = std::mem::replace(&mut app.ai.ask.input, question.to_string());
    let r = send(app);
    let st = &mut app.ai.ask;
    if r.is_ok() {
        if let Some(Entry::Question(q)) = st.entries.iter_mut().rev().find(|e| matches!(e, Entry::Question(_))) {
            *q = shown.to_string();
        }
        if let Some(c) = st.current.as_mut() {
            c.question = typed.clone();
        }
    }
    st.input = typed;
    r
}

/// Send the conversation as it stands; the reply comes in through [`poll`].
fn request(app: &mut VectorcraftApp) -> Result<(), String> {
    let key = super::account::key_to_send(app);
    let send = if app.ai.ask.mode == Mode::Muslin { pf_ai::muslin::request } else { ask::request };
    let http = match send(&key, &app.ai.ask.messages) {
        Ok(h) => h,
        Err(e) => return give_up(app, e.to_string()),
    };
    if let Some(c) = app.ai.ask.current.as_mut() {
        c.rounds += 1;
    }
    let rx = match super::post_ai(app, http, "ask-about-patterns", cloud::MESSAGES, Vec::new(), ask::parse_reply) {
        Ok(rx) => rx,
        Err(e) => return give_up(app, e),
    };
    app.ai.ask.pending = Some(Pending { rx, since: app.ai.now });
    Ok(())
}

/// Start a muslin check (Check My Muslin): a new conversation in [`Mode::Muslin`] whose first
/// question is `message` ([`pf_ai::muslin::first_message`]), shown as `shown`.
pub fn begin_check(app: &mut VectorcraftApp, message: Value, shown: String) -> Result<(), String> {
    if app.ai.ask.asking() {
        return Err("already asking: wait for the answer".into());
    }
    clear(app);
    super::account::key_to_send(app); // (reads the account once)
    let st = &mut app.ai.ask;
    st.open = true;
    st.mode = Mode::Muslin;
    if let Err(m) = super::account::require_account(app) {
        super::account::show_if_needed(app);
        app.ai.ask.error = Some(m.clone());
        return Err(m);
    }
    let st = &mut app.ai.ask;
    st.current = Some(Current { question: String::new(), start: 0, rounds: 0 });
    st.messages.push(message);
    st.entries.push(Entry::Question(shown));
    st.error = None;
    request(app)
}

/// The question got no answer: the conversation goes back to before it (its question stays in
/// the box to ask again; changes already made stay, each undoable), and `m` is shown.
fn give_up(app: &mut VectorcraftApp, m: String) -> Result<(), String> {
    let st = &mut app.ai.ask;
    st.pending = None;
    st.approval = None;
    if let Some(c) = st.current.take() {
        st.messages.truncate(c.start);
        st.input = c.question;
        if let Some(i) = st.entries.iter().rposition(|e| matches!(e, Entry::Question(_))) {
            st.entries.remove(i);
        }
    }
    st.error = Some(m.clone());
    Err(m)
}

/// Each frame: take in the reply that arrived, or why none did.
pub fn poll(app: &mut VectorcraftApp, ctx: &egui::Context) {
    let Some(p) = app.ai.ask.pending.as_ref() else { return };
    let r = match p.rx.try_recv() {
        Ok(r) => r,
        Err(TryRecvError::Empty) => {
            // The thread doesn't wake the window: look again shortly.
            ctx.request_repaint_after(std::time::Duration::from_millis(250));
            return;
        }
        Err(TryRecvError::Disconnected) => Err(AiError::Network("the request stopped".into())),
    };
    app.ai.ask.pending = None;
    // the credits it cost show next time the account is read
    app.ai.account.stale |= super::account::paying_with_credits(app);
    let reply = match r {
        Ok(reply) => reply,
        Err(e) => {
            let _ = give_up(app, e.to_string()); // (shown in the window)
            return;
        }
    };
    let st = &mut app.ai.ask;
    st.messages.push(reply.message);
    if !reply.text.is_empty() {
        st.entries.push(Entry::Answer(reply.text));
    }
    if reply.calls.is_empty() {
        st.current = None;
        st.input.clear();
        return;
    }
    if st.current.as_ref().is_some_and(|c| c.rounds >= ask::MAX_ROUNDS) {
        let _ = give_up(app, "Claude took too many steps: ask again, more simply.".into());
        return;
    }
    run_calls(app, &reply.calls);
}

/// Answer one reply's tool calls: searches and reads at once, changes after the user's Allow.
fn run_calls(app: &mut VectorcraftApp, calls: &[ToolCall]) {
    let mut results = Vec::new();
    let mut changes = Vec::new();
    for call in calls {
        let result = match call.name.as_str() {
            ask::FIND => {
                let q = call.input.get("query").and_then(Value::as_str).unwrap_or_default();
                Some(Ok(find(app, q).to_string()))
            }
            ask::RUN => {
                let id = call.input.get("command").and_then(Value::as_str).unwrap_or_default();
                let params = call.input.get("params").cloned().unwrap_or_else(|| json!({}));
                match vectorcraft_engine::find_command(id).filter(|_| !blocked(id)) {
                    None if blocked(id) => Some(to_claude(format!(
                        "{id} can't be run from here: files, preferences, plug-ins and windows are out of reach; tell the user where to find it instead"
                    ))),
                    None => Some(to_claude(format!("there is no command {id:?}: look for one with {}", ask::FIND))),
                    Some(_) if !params.is_object() => Some(to_claude("params must be an object".to_string())),
                    Some(spec) if access(spec) == Access::Blocked || names_a_file(&params) => Some(to_claude(format!(
                        "{id} can't be run from here: files, preferences, plug-ins and windows are out of reach; tell the user where to find it instead"
                    ))),
                    Some(spec) if access(spec) == Access::Read => {
                        Some(app.session.execute(id, &params).map(|v| v.to_string()).map_err(|e| e.to_string()))
                    }
                    Some(spec) if access(spec) == Access::Personal => {
                        changes.push((call.id.clone(), id.to_string(), params, tl!("Read your measurements").to_string()));
                        None
                    }
                    Some(spec) => {
                        changes.push((call.id.clone(), id.to_string(), params, describe(spec)));
                        None
                    }
                }
            }
            other => Some(to_claude(format!("there is no tool {other:?}"))),
        };
        results.push((call.id.clone(), result));
    }
    if changes.is_empty() {
        send_results(app, results);
    } else {
        app.ai.ask.approval = Some(Approval { changes, results });
    }
}

/// The user's answer to the changes waiting (Allow / Don't Allow): made or declined, then the
/// results go back to Claude.
pub fn decide(app: &mut VectorcraftApp, allow: bool) {
    let Some(Approval { changes, mut results }) = app.ai.ask.approval.take() else { return };
    for (call, id, params, label) in changes {
        let r = if allow {
            let r = app.run(&id, params);
            let note = match &r {
                Ok(_) => format!("{} {label}", tl!("Done:")),
                Err(e) => format!("{} {label}: {}", tl!("Couldn't do:"), crate::i18n::msg(e)),
            };
            app.ai.ask.entries.push(Entry::Note(note));
            r.map(|v| v.to_string())
        } else {
            app.ai.ask.entries.push(Entry::Note(format!("{} {label}", tl!("Not allowed:"))));
            to_claude("The user declined this.".to_string())
        };
        if let Some(slot) = results.iter_mut().find(|(c, _)| *c == call) {
            slot.1 = Some(r);
        }
    }
    send_results(app, results);
}

/// Send a round's results back and wait for Claude to go on.
fn send_results(app: &mut VectorcraftApp, results: Vec<(String, Option<Result<String, String>>)>) {
    let results: Vec<ToolResult> =
        results.into_iter().map(|(id, r)| ToolResult { id, result: r.unwrap_or_else(|| to_claude("not run".to_string())) }).collect();
    app.ai.ask.messages.push(ask::tool_results(&results));
    let _ = request(app); // (a failure is shown in the window)
}

/// Start a new conversation (New Conversation): the answers so far, and any question on its way,
/// are dropped.
pub fn clear(app: &mut VectorcraftApp) {
    let st = &mut app.ai.ask;
    st.entries.clear();
    st.messages.clear();
    st.mode = Mode::Ask;
    st.current = None;
    st.pending = None;
    st.approval = None;
    st.error = None;
}

/// The window, while open.
pub fn window(app: &mut VectorcraftApp, ctx: &egui::Context) {
    if !app.ai.ask.open {
        return;
    }
    let t = Tokens::get(ctx);
    let now = app.ai.now;
    let mut open = true;
    let mut ask_now = false;
    let mut new_conversation = false;
    let mut decision = None;
    let title = if app.ai.ask.mode == Mode::Muslin { tl!("Check My Muslin") } else { tl!("Ask About Patterns") };
    egui::Window::new(title)
        .id(egui::Id::new("ask-about-patterns"))
        .open(&mut open)
        .collapsible(false)
        .resizable(true)
        .default_size([460.0, 560.0])
        .default_pos(ctx.content_rect().center() - egui::vec2(230.0, 280.0))
        .show(ctx, |ui| {
            let st = &app.ai.ask;
            // room under the conversation for the question box, its buttons, the key and the note
            let footer = 210.0;
            egui::ScrollArea::vertical()
                .id_salt("ask-conversation")
                .max_height((ui.available_height() - footer).max(120.0))
                .stick_to_bottom(true)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    if st.entries.is_empty() {
                        ui.label(RichText::new(tl!("Ask anything about sewing patterns or PatternFoundry: how much ease to add, how to move a dart, which block to start from, where a feature is. Claude can also draft and change your pattern when you ask, after you allow it.")).color(t.text_dim));
                    }
                    for e in &st.entries {
                        ui.add_space(4.0);
                        match e {
                            Entry::Question(q) => {
                                ui.add_space(4.0);
                                ui.label(RichText::new(q).strong());
                            }
                            Entry::Answer(a) => {
                                ui.label(a);
                            }
                            Entry::Note(n) => {
                                ui.label(RichText::new(n).size(12.0).color(t.text_dim));
                            }
                        }
                    }
                    if let Some(a) = &st.approval {
                        ui.add_space(6.0);
                        egui::Frame::group(ui.style()).show(ui, |ui| {
                            ui.label(RichText::new(tl!("Claude asks to:")).strong());
                            for (_, _, _, label) in &a.changes {
                                ui.label(format!("- {label}"));
                            }
                            ui.horizontal(|ui| {
                                if crate::widgets::primary_button(ui, tl!("Allow")).clicked() {
                                    decision = Some(true);
                                }
                                if ui.button(tl!("Don't Allow")).clicked() {
                                    decision = Some(false);
                                }
                            });
                            ui.label(RichText::new(tl!("Edit › Undo takes each change back.")).size(11.0).color(t.text_dim));
                        });
                    } else if let Some(secs) = st.waiting_for(now) {
                        ui.horizontal(|ui| {
                            ui.add(egui::Spinner::new());
                            ui.label(format!("{} {:.0} s", tl!("Thinking…"), secs));
                        });
                    }
                });
            ui.separator();
            if let Some(e) = &st.error {
                ui.label(RichText::new(crate::i18n::msg(e)).color(t.error));
            }
            let st = &mut app.ai.ask;
            let r = ui.add(
                egui::TextEdit::multiline(&mut st.input)
                    .desired_rows(2)
                    .desired_width(f32::INFINITY)
                    .hint_text(tl!("Type your question, then press Enter")),
            );
            // Enter asks; Shift+Enter starts a new line.
            if r.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter) && !i.modifiers.shift) {
                st.input = st.input.trim_end_matches('\n').to_string();
                ask_now = true;
            }
            ui.horizontal(|ui| {
                let can = !st.asking();
                if ui.add_enabled_ui(can, |ui| crate::widgets::primary_button(ui, tl!("Ask"))).inner.clicked() {
                    ask_now = true;
                }
                if ui.button(tl!("New Conversation")).clicked() {
                    new_conversation = true;
                }
            });
            ui.checkbox(&mut st.include_pattern, tl!("Include my pattern"))
                .on_hover_text(tl!("Its pieces and sizes, and which measurements are set: never their values"));
            super::account::paid_line(app, ui, &t);
            ui.label(RichText::new(tl!("Answers come from Claude. They can be wrong: sew a muslin before cutting good fabric.")).size(11.0).color(t.text_dim));
        });
    if let Some(allow) = decision {
        decide(app, allow);
    }
    if new_conversation {
        clear(app);
    }
    if ask_now && !app.ai.ask.asking() {
        // (a failure is shown in the window)
        let _ = send(app);
    }
    if !open {
        app.ai.ask.open = false;
    }
}
