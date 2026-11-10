//! Ask About Patterns and Foundry Account (PF-8.1), with a fake AI service: the conversation, the
//! pattern summary (measurement names, never values), commands run at once or after Allow,
//! the web's fetch, and requests paid with Foundry credits.

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::{Value, json};
use vectorcraft_engine::Session;

use super::ask::{self, Access, Entry};
use crate::{HttpRequest, SecretStore, Services, VectorcraftApp};

type Sent = Arc<Mutex<Vec<HttpRequest>>>;

/// Secrets kept in memory.
#[derive(Clone, Default)]
struct Keys(Arc<Mutex<HashMap<String, String>>>);

impl SecretStore for Keys {
    fn load(&self, name: &str) -> Option<String> {
        self.0.lock().unwrap().get(name).cloned()
    }
    fn save(&self, name: &str, value: Option<&str>) -> Result<(), String> {
        let mut m = self.0.lock().unwrap();
        match value {
            Some(v) => m.insert(name.to_string(), v.to_string()),
            None => m.remove(name),
        };
        Ok(())
    }
}

/// An app whose AI service answers with `replies` in turn, recording what it was sent, signed in
/// to a Foundry service.
fn app(replies: Vec<(u16, String)>) -> (VectorcraftApp, Sent, Keys) {
    let sent: Sent = Arc::default();
    let keys = Keys::default();
    keys.save(pf_ai::cloud::TOKEN_NAME, Some("sfs_session")).unwrap();
    keys.save("foundry-server", Some("https://cloud.example.com")).unwrap();
    let queue = Arc::new(Mutex::new(VecDeque::from(replies)));
    let log = sent.clone();
    let services = Services {
        http_post: Some(Arc::new(move |r: &HttpRequest| {
            log.lock().unwrap().push(r.clone());
            Ok(queue.lock().unwrap().pop_front().unwrap_or((500, String::new())))
        })),
        secrets: Some(Box::new(keys.clone())),
        ..Default::default()
    };
    let mut app = VectorcraftApp::new(Session::new(), services);
    app.run("file.new", json!({"width": 1400, "height": 1000})).unwrap();
    (app, sent, keys)
}

fn reply(stop: &str, content: Value) -> (u16, String) {
    (200, json!({"type": "message", "model": "claude-opus-5-5", "content": content, "stop_reason": stop}).to_string())
}

fn answer(text: &str) -> (u16, String) {
    reply("end_turn", json!([{"type": "text", "text": text}]))
}

fn body(sent: &Sent, i: usize) -> Value {
    serde_json::from_str(&sent.lock().unwrap()[i].body).unwrap()
}

/// Poll as frames do until the reply is in (or something waits for the user).
fn wait(app: &mut VectorcraftApp) {
    let ctx = egui::Context::default();
    for _ in 0..1000 {
        ask::poll(app, &ctx);
        if app.ai.ask.waiting_for(0.0).is_none() {
            return;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    panic!("no answer");
}

/// Every piece of text the Ask window paints.
fn window_text(app: &mut VectorcraftApp) -> Vec<String> {
    let ctx = egui::Context::default();
    crate::theme::install_fonts(&ctx);
    let raw = || egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1200.0, 900.0))), ..Default::default() };
    for _ in 0..3 {
        ctx.run_ui(raw(), |ui| ask::window(app, &ui.ctx().clone())).textures_delta.clear();
    }
    let out = ctx.run_ui(raw(), |ui| ask::window(app, &ui.ctx().clone()));
    fn walk(s: &egui::Shape, into: &mut Vec<String>) {
        match s {
            egui::Shape::Text(t) => into.push(t.galley.text().to_string()),
            egui::Shape::Vec(v) => v.iter().for_each(|s| walk(s, into)),
            _ => {}
        }
    }
    let mut texts = Vec::new();
    out.shapes.iter().for_each(|c| walk(&c.shape, &mut texts));
    texts
}

/// The skirt block drafted from the user's measurements and made pattern pieces.
fn skirt(app: &mut VectorcraftApp) {
    app.run("measure.set", json!({"name": "Me", "values": {"waist": 683, "hip": 941, "waist_to_hip": 201, "skirt_length": 607}})).unwrap();
    app.run("block.skirt", json!({"origin": [40, 40]})).unwrap();
    app.run("select.all", json!({})).unwrap();
    app.run("piece.make", json!({"allowance": 15})).unwrap();
}

#[test]
fn a_question_goes_with_the_pattern_but_never_the_measured_values() {
    let (mut app, sent, _) = app(vec![answer("Add about 2 cm of ease at the hip."), answer("Then 1 cm.")]);
    assert!(crate::menus::enabled(&app, "ui.askAboutPatterns"));
    skirt(&mut app);
    let r = app.run("ui.askAboutPatterns", json!({"question": "How much ease for a straight skirt?"})).unwrap();
    assert_eq!(r, json!({"open": true, "asking": true}));
    wait(&mut app);
    let st = &app.ai.ask;
    assert!(st.open && st.input.is_empty() && st.error.is_none() && !st.asking(), "{:?}", st.error);
    assert_eq!(
        st.entries,
        [Entry::Question("How much ease for a straight skirt?".into()), Entry::Answer("Add about 2 cm of ease at the hip.".into())]
    );
    let texts = window_text(&mut app);
    for want in ["Ask About Patterns", "How much ease for a straight skirt?", "Add about 2 cm of ease at the hip.", "Ask", "Include my pattern"] {
        assert!(texts.iter().any(|t| t == want), "{want} missing from {texts:?}");
    }
    // the pattern went along: its pieces, its block and the measurements' names, not their values
    let b = body(&sent, 0);
    let summary = b["messages"][0]["content"][0]["text"].as_str().unwrap();
    assert!(summary.starts_with("The pattern open now") && summary.contains("2 pattern pieces") && summary.contains("Skirt Block"), "{summary}");
    assert!(summary.contains("skirt") && summary.contains("waist_to_hip") && summary.contains("\"Me\""), "{summary}");
    for value in ["683", "941", "201", "607"] {
        assert!(!summary.contains(value), "the measured value {value} went along: {summary}");
    }
    assert!(!sent.lock().unwrap()[0].body.contains("683"));
    // without the pattern, only the question
    app.ai.ask.include_pattern = false;
    app.ai.ask.input = "And for a pencil skirt?".into();
    ask::send(&mut app).unwrap();
    wait(&mut app);
    let b2 = body(&sent, 1);
    let m = b2["messages"].as_array().unwrap();
    assert_eq!(m.len(), 3);
    assert_eq!(m[2]["content"], json!([{"type": "text", "text": "And for a pencil skirt?"}]));
    assert_eq!(b2["system"], b["system"], "the instructions stay the same");
    ask::clear(&mut app);
    assert!(app.ai.ask.entries.is_empty());
}

#[test]
fn claude_reads_at_once_and_drafts_and_reads_measurements_once_allowed() {
    let (mut app, sent, _) = app(vec![
        reply(
            "tool_use",
            json!([
                {"type": "thinking", "thinking": "", "signature": "s1"},
                {"type": "tool_use", "id": "t1", "name": "find_commands", "input": {"query": "skirt block"}},
                {"type": "tool_use", "id": "t2", "name": "run_command", "input": {"command": "block.measurements", "params": {"block": "skirt"}}},
            ]),
        ),
        reply(
            "tool_use",
            json!([
                {"type": "text", "text": "I'll draft it from your measurements."},
                {"type": "tool_use", "id": "t3", "name": "run_command", "input": {"command": "measure.info", "params": {}}},
                {"type": "tool_use", "id": "t4", "name": "run_command", "input": {"command": "block.skirt", "params": {"origin": [40, 40]}}},
                {"type": "tool_use", "id": "t5", "name": "run_command", "input": {"command": "file.save", "params": {}}},
            ]),
        ),
        answer("Done: the skirt block is drafted."),
    ]);
    app.run("measure.set", json!({"values": {"waist": 683, "hip": 941, "waist_to_hip": 201, "skirt_length": 607}})).unwrap();
    app.run("ui.askAboutPatterns", json!({"question": "Draft me a skirt"})).unwrap();
    wait(&mut app);
    // the search and the read ran at once and went back
    let b = body(&sent, 1);
    assert_eq!(b["messages"][1]["content"][0]["signature"], "s1", "the reply went back unchanged");
    let results = b["messages"][2]["content"].as_array().unwrap();
    let found: Value = serde_json::from_str(results[0]["content"].as_str().unwrap()).unwrap();
    assert!(found.as_array().unwrap().iter().any(|c| c["id"] == "block.skirt" && c["changesThePattern"] == true), "{found}");
    assert!(results[1]["content"].as_str().unwrap().contains("waist_to_hip"), "{results:?}");
    // reading the measurements and drafting wait for the user
    let a = app.ai.ask.approval.as_ref().expect("waits for the user");
    assert_eq!(a.changes.iter().map(|c| c.1.as_str()).collect::<Vec<_>>(), ["measure.info", "block.skirt"]);
    let texts = window_text(&mut app);
    assert!(
        texts.iter().any(|t| t == "Claude asks to:") && texts.iter().any(|t| t == "- Read your measurements") && texts.iter().any(|t| t == "Allow"),
        "{texts:?}"
    );
    let blocks = |app: &VectorcraftApp| {
        let mut n = 0;
        app.session.active().unwrap().doc.walk(|x| n += usize::from(x.extras().block.is_some()));
        n
    };
    assert_eq!(blocks(&app), 0, "nothing is drafted before Allow");
    assert!(!sent.lock().unwrap().iter().any(|r| r.body.contains("683")), "no measured value has gone yet");
    super::ask::decide(&mut app, true);
    wait(&mut app);
    assert_eq!(blocks(&app), 2, "the back and the front");
    // the measurements went once allowed, with the draft's result and the blocked file command
    let b = body(&sent, 2);
    let results = b["messages"][4]["content"].as_array().unwrap();
    assert_eq!(results.len(), 3);
    assert!(results[0]["content"].as_str().unwrap().contains("683"));
    assert!(results[1].get("is_error").is_none(), "{results:?}");
    assert_eq!(results[2]["is_error"], true);
    assert!(results[2]["content"].as_str().unwrap().contains("can't be run from here"));
    let st = &app.ai.ask;
    assert!(!st.asking());
    assert!(st.entries.iter().any(|e| matches!(e, Entry::Note(n) if n.starts_with("Done:"))), "{:?}", st.entries);
    assert_eq!(st.entries.last(), Some(&Entry::Answer("Done: the skirt block is drafted.".into())));
    app.run("edit.undo", json!({})).unwrap();
    assert_eq!(blocks(&app), 0, "one undo takes the draft back");
}

#[test]
fn a_declined_change_is_not_made() {
    let (mut app, sent, _) = app(vec![
        reply("tool_use", json!([{"type": "tool_use", "id": "t1", "name": "run_command", "input": {"command": "measure.info", "params": {}}}])),
        answer("Then tell me your waist and hip."),
    ]);
    app.run("measure.set", json!({"values": {"waist": 683}})).unwrap();
    app.run("ui.askAboutPatterns", json!({"question": "Does my skirt fit?"})).unwrap();
    wait(&mut app);
    super::ask::decide(&mut app, false);
    wait(&mut app);
    let b = body(&sent, 1);
    assert_eq!(b["messages"][2]["content"][0]["content"], "The user declined this.");
    assert!(app.ai.ask.entries.iter().any(|e| matches!(e, Entry::Note(n) if n.starts_with("Not allowed:"))));
    assert!(!sent.lock().unwrap().iter().any(|r| r.body.contains("683")));
}

#[test]
fn what_claude_may_run() {
    let access = |id: &str| ask::access(vectorcraft_engine::find_command(id).unwrap());
    for id in ["piece.list", "piece.info", "grade.info", "document.node", "select.all", "block.measurements", "design.length"] {
        assert_eq!(access(id), Access::Read, "{id}");
    }
    for id in ["block.skirt", "piece.make", "design.rotateDart", "measure.set", "grade.setRule", "edit.undo"] {
        assert_eq!(access(id), Access::Change, "{id}");
    }
    assert_eq!(access("measure.info"), Access::Personal);
    for id in ["prefs.set", "plugin.install", "document.export", "text.addFontFiles", "document.exportForScreens"] {
        assert_eq!(access(id), Access::Blocked, "{id}");
    }
    assert!(ask::blocked("file.save") && ask::blocked("ui.askAboutPatterns") && !ask::blocked("block.skirt"));
}

#[test]
fn without_an_account_or_a_question_nothing_is_sent() {
    let (mut app, sent, keys) = app(vec![answer("Use 1.5 cm seams.")]);
    keys.save(pf_ai::cloud::TOKEN_NAME, None).unwrap();
    app.run("ui.askAboutPatterns", json!({})).unwrap();
    app.ai.ask.input = "Where do I add seam allowance?".into();
    assert_eq!(ask::send(&mut app).unwrap_err(), "AI now uses your Foundry Account. Sign in to continue.");
    assert!(app.ai.account.open, "the Foundry Account window opens to sign in");
    app.ai.ask.input = "   ".into();
    assert_eq!(ask::send(&mut app).unwrap_err(), "AI now uses your Foundry Account. Sign in to continue.");
    assert!(sent.lock().unwrap().is_empty());
    // signed in: an empty question is still refused, a real one goes to the service
    app.ai.account.token = "sfs_session".into();
    assert_eq!(ask::send(&mut app).unwrap_err(), "Type a question first.");
    app.ai.ask.input = "Where do I add seam allowance?".into();
    ask::send(&mut app).unwrap();
    assert!(ask::send(&mut app).is_err(), "one question at a time");
    wait(&mut app);
    assert_eq!(sent.lock().unwrap().len(), 1);
    assert_eq!(sent.lock().unwrap()[0].url, "https://cloud.example.com/v1/messages");
    // a build without a way to reach the service
    let mut bare = VectorcraftApp::new(Session::new(), Services::default());
    assert!(!crate::menus::enabled(&bare, "ui.askAboutPatterns") && !crate::menus::enabled(&bare, "ui.account"));
    assert!(bare.run("ui.askAboutPatterns", json!({})).is_err());
}

/// The web: the browser's fetch answers later, on the UI thread, and the session stays in its store.
#[test]
fn the_web_sends_through_the_browser() {
    let sent: Sent = Arc::default();
    let log = sent.clone();
    type Done = Box<dyn FnOnce(Result<(u16, String), String>)>;
    let later: std::rc::Rc<std::cell::RefCell<Vec<Done>>> = Default::default();
    let queue = later.clone();
    let keys = Keys::default();
    keys.save(pf_ai::cloud::TOKEN_NAME, Some("sfs_web")).unwrap();
    keys.save("foundry-server", Some("https://cloud.example.com")).unwrap();
    let services = Services {
        http_post_async: Some(std::rc::Rc::new(move |r: HttpRequest, done| {
            log.lock().unwrap().push(r);
            queue.borrow_mut().push(done);
        })),
        secrets: Some(Box::new(keys.clone())),
        ..Default::default()
    };
    let mut app = VectorcraftApp::new(Session::new(), services);
    app.run("file.new", json!({"width": 400, "height": 300})).unwrap();
    app.run("ui.askAboutPatterns", json!({"question": "What is a toile?"})).unwrap();
    assert_eq!(sent.lock().unwrap().len(), 1);
    assert_eq!(sent.lock().unwrap()[0].url, "https://cloud.example.com/v1/messages");
    assert!(app.ai.ask.waiting_for(0.0).is_some(), "waiting for the browser");
    let done = later.borrow_mut().pop().unwrap();
    done(Ok(answer("A test garment in cheap fabric.")));
    ask::poll(&mut app, &egui::Context::default());
    assert_eq!(app.ai.ask.entries.last(), Some(&Entry::Answer("A test garment in cheap fabric.".into())));
}

/// #140: the Foundry Account is the only way to pay for AI. The windows have no API key option, a
/// key kept by an older version is deleted on load (and never sent), and every request goes to the
/// account service.
#[test]
fn the_window_has_no_api_key_option_and_a_stored_key_is_deleted() {
    let (mut app, sent, keys) = app(vec![answer("Use 1.5 cm seams.")]);
    keys.save(super::LEGACY_KEY_NAME, Some("sk-ant-old")).unwrap();
    keys.save(super::LEGACY_CREDITS_NAME, Some("0")).unwrap();
    app.run("ui.askAboutPatterns", json!({"question": "Seam allowance?"})).unwrap();
    assert_eq!(keys.load(super::LEGACY_KEY_NAME), None, "the old key is deleted from the store");
    assert_eq!(keys.load(super::LEGACY_CREDITS_NAME), None);
    assert!(app.ai.account.legacy_key_removed);
    wait(&mut app);
    {
        let sent = sent.lock().unwrap();
        assert_eq!(sent.len(), 1);
        assert_eq!(sent[0].url, "https://cloud.example.com/v1/messages", "AI requests go to the account service only");
        assert!(sent.iter().all(|r| r.headers.iter().all(|h| h.0 != "x-api-key") && !r.body.contains("sk-ant-old")));
    }
    let shown = window_text(&mut app).join("\n").to_lowercase();
    for gone in ["api key", "anthropic", "remember the key", "get a key", "change key"] {
        assert!(!shown.contains(gone), "{gone:?} in the Ask window: {shown}");
    }
    // signed out: the same, plus the one note
    let (mut app, _, keys) = self::app(vec![]);
    keys.save(pf_ai::cloud::TOKEN_NAME, None).unwrap();
    keys.save(super::LEGACY_KEY_NAME, Some("sk-ant-old")).unwrap();
    app.run("ui.askAboutPatterns", json!({})).unwrap();
    let shown = window_text(&mut app).join("\n");
    assert!(shown.contains("AI now uses your Foundry Account. Sign in to continue."), "{shown}");
    assert!(!shown.to_lowercase().contains("api key"), "{shown}");
    assert_eq!(keys.load(super::LEGACY_KEY_NAME), None);
}

#[test]
fn paid_with_credits_the_request_goes_to_the_foundry_service() {
    let (mut app, sent, keys) = app(vec![answer("Use 1.5 cm seams."), (402, json!({"error": {"message": "out of credits"}}).to_string())]);
    keys.save(pf_ai::cloud::TOKEN_NAME, Some("sfs_session")).unwrap();
    keys.save("foundry-server", Some("https://cloud.example.com")).unwrap();
    app.run("ui.askAboutPatterns", json!({"question": "Seam allowance?"})).unwrap();
    wait(&mut app);
    let r = sent.lock().unwrap()[0].clone();
    assert_eq!(r.url, "https://cloud.example.com/v1/messages");
    assert!(r.headers.contains(&("authorization", "Bearer sfs_session".to_string())));
    assert!(r.headers.iter().all(|h| h.0 != "x-api-key"), "the user's own key isn't sent");
    assert_eq!(app.ai.ask.entries.last(), Some(&Entry::Answer("Use 1.5 cm seams.".into())));
    // out of credits: the service's own refusal
    app.ai.ask.input = "And for knits?".into();
    ask::send(&mut app).unwrap();
    wait(&mut app);
    assert_eq!(app.ai.ask.error.as_deref(), Some("Not enough Foundry credits: buy more in Foundry Account."));
    assert!(window_text(&mut app).iter().any(|t| t.starts_with("Paid with Foundry credits")), "the window says how it is paid");
}

/// An account whose email isn't confirmed: the service's 403 says where the link went, and the
/// Ask window shows those words (not "Sign in"), still signed in; a 403 without a message still
/// asks to sign in.
#[test]
fn an_unconfirmed_email_shows_the_services_words_not_sign_in() {
    let words = "Confirm your email first: we sent a link to d@e.f (send it again from your Foundry Account window).";
    let (mut app, _, keys) =
        app(vec![(403, json!({"type": "error", "error": {"type": "permission_error", "message": words}}).to_string()), (403, "{}".to_string())]);
    keys.save(pf_ai::cloud::TOKEN_NAME, Some("sfs_session")).unwrap();
    keys.save("foundry-server", Some("https://cloud.example.com")).unwrap();
    app.run("ui.askAboutPatterns", json!({"question": "Seam allowance?"})).unwrap();
    wait(&mut app);
    assert_eq!(app.ai.ask.error.as_deref(), Some(words));
    assert!(window_text(&mut app).iter().any(|t| t == words), "the window shows the service's words");
    app.ai.ask.input = "And for knits?".into();
    ask::send(&mut app).unwrap();
    wait(&mut app);
    assert!(app.ai.ask.error.as_deref().is_some_and(|e| e.starts_with("Sign in to your Foundry account")), "{:?}", app.ai.ask.error);
}

/// A small PNG standing in for a photo of a muslin.
fn photo_png(w: u32, h: u32) -> Vec<u8> {
    let img = image::RgbImage::from_fn(w, h, |x, y| image::Rgb([(x % 256) as u8, (y % 256) as u8, 128]));
    let mut out = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgb8(img).write_to(&mut out, image::ImageFormat::Png).unwrap();
    out.into_inner()
}

#[test]
fn a_muslin_check_sends_the_photos_and_the_pattern_and_alters_it_once_allowed() {
    let (mut app, sent, _) = app(vec![
        reply(
            "tool_use",
            json!([
                {"type": "text", "text": "The back is tight across the hips: let out the side seams."},
                {"type": "tool_use", "id": "t1", "name": "run_command", "input": {"command": "measure.set", "params": {"values": {"hip": 960}}}},
            ]),
        ),
        answer("Done: the hip is 960 mm now, and the skirt is redrafted."),
    ]);
    assert!(crate::menus::enabled(&app, "ui.checkMuslin"));
    skirt(&mut app);
    app.run("ui.checkMuslin", json!({})).unwrap();
    // A large photo is made smaller; a file that isn't a picture is refused.
    super::muslin::add_photo(&mut app, "front.png", &photo_png(2400, 1600));
    super::muslin::add_photo(&mut app, "notes.txt", b"not a photo");
    let st = &app.ai.muslin;
    assert_eq!(st.photos.len(), 1);
    assert_eq!(st.photos[0].size, (pf_ai::IMAGE_SIDE, pf_ai::IMAGE_SIDE * 2 / 3));
    assert!(st.error.as_deref().unwrap().contains("can't be read"));
    app.ai.muslin.fabric = "calico".into();
    app.ai.muslin.feels[2] = true;
    app.run("ui.checkMuslin", json!({"check": true})).unwrap();
    assert!(app.ai.muslin.photos.is_empty() && !app.ai.muslin.open, "the photos are let go of once sent");
    wait(&mut app);
    // The photo, the pattern (names, no measured values) and the answers went, with the fitting
    // instructions.
    let b = body(&sent, 0);
    let content = b["messages"][0]["content"].as_array().unwrap();
    assert_eq!(content.iter().filter(|c| c["type"] == "image").count(), 1);
    assert_eq!(content[1]["source"]["media_type"], "image/jpeg");
    let text: Vec<&str> = content.iter().filter_map(|c| c["text"].as_str()).collect();
    assert!(
        text.iter().any(|t| t.contains("Skirt Block"))
            && text.iter().any(|t| t.contains("Muslin fabric: calico") && t.contains("tight at the waist or hips")),
        "{text:?}"
    );
    assert!(!sent.lock().unwrap()[0].body.contains("941"), "no measured value went along");
    assert!(b["system"].as_str().unwrap().contains("drag lines"));
    // The answer is in the Ask window, titled for the check; the alteration waits for Allow.
    let texts = window_text(&mut app);
    assert!(texts.iter().any(|t| t == "Check My Muslin") && texts.iter().any(|t| t == "My muslin (1 photos)"), "{texts:?}");
    assert_eq!(app.ai.ask.approval.as_ref().unwrap().changes[0].1, "measure.set");
    super::ask::decide(&mut app, true);
    wait(&mut app);
    let hip = app.session.active().unwrap().doc.measurements.as_deref().unwrap().values["hip"];
    assert_eq!(hip, 960.0);
    // The follow-up goes with the same fitting instructions.
    assert!(body(&sent, 1)["system"].as_str().unwrap().contains("drag lines"));
    // A new conversation is an Ask again.
    super::ask::clear(&mut app);
    assert_eq!(app.ai.ask.mode, super::ask::Mode::Ask);
}

#[test]
fn a_muslin_check_needs_a_photo() {
    let (mut app, sent, _) = app(vec![]);
    app.run("ui.checkMuslin", json!({})).unwrap();
    assert_eq!(app.run("ui.checkMuslin", json!({"check": true})).unwrap_err(), "Add a photo of the muslin first.");
    assert!(sent.lock().unwrap().is_empty());
    // Closing the window lets the photos go.
    super::muslin::add_photo(&mut app, "front.png", &photo_png(40, 60));
    assert_eq!(app.ai.muslin.photos.len(), 1);
    let mut bare = VectorcraftApp::new(Session::new(), Services::default());
    assert!(!crate::menus::enabled(&bare, "ui.checkMuslin") && bare.run("ui.checkMuslin", json!({})).is_err());
}

/// Publish to MakerFoundry (PF-11): poll as frames do until MakerFoundry's answer is in.
fn wait_published(app: &mut VectorcraftApp) {
    let ctx = egui::Context::default();
    for _ in 0..1000 {
        super::publish::poll(app, &ctx);
        if app.ai.publish.made.is_some() || app.ai.publish.error.is_some() {
            return;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    panic!("no answer from MakerFoundry");
}

/// An app with a rectangle drawn, signed in to a Foundry Account, MakerFoundry answering `replies`.
fn signed_in(replies: Vec<(u16, String)>) -> (VectorcraftApp, Sent) {
    let (mut app, sent, keys) = app(replies);
    keys.save(pf_ai::cloud::TOKEN_NAME, Some("sfs_session")).unwrap();
    app.run("shape.rectangle", json!({"x": 10, "y": 10, "width": 100, "height": 60})).unwrap();
    (app, sent)
}

#[test]
fn a_design_is_shown_before_it_goes_and_sent_as_svg_with_the_session_alone() {
    let draft = json!({"id": 42, "url": "https://makerfoundry.app/designs/42/edit", "kind": "laser", "format": "svg"});
    let (mut app, sent) = signed_in(vec![(201, draft.to_string())]);
    // Opening it shows what goes and where; nothing is sent.
    let r = app.run("ui.publishToMakerFoundry", json!({})).unwrap();
    assert_eq!(
        (r["open"].as_bool(), r["sentTo"].as_str(), r["signedIn"].as_bool()),
        (Some(true), Some("https://makerfoundry.app/api/v1/designs"), Some(true))
    );
    assert_eq!(r["fileName"], "Untitled-1.svg", "{r}");
    assert!(r["bytes"].as_u64().unwrap() > 0);
    assert!(sent.lock().unwrap().is_empty(), "nothing goes before Publish");
    // Named, then sent.
    app.run("ui.publishToMakerFoundry", json!({"send": true, "name": "A-line skirt (v2)"})).unwrap();
    wait_published(&mut app);
    let req = sent.lock().unwrap()[0].clone();
    assert_eq!(req.url, "https://makerfoundry.app/api/v1/designs");
    assert!(req.headers.contains(&("authorization", "Bearer sfs_session".to_string())));
    assert!(req.headers.iter().all(|h| h.0 != "x-api-key" && !h.1.contains("sk-ant")), "the Anthropic key never goes to MakerFoundry");
    let b = body(&sent, 0);
    assert_eq!(b["file_name"], "A-line_skirt_v2.svg");
    assert!(b["app"].as_str().unwrap().starts_with("PatternFoundry "), "{b}");
    let svg = String::from_utf8(vectorcraft_format::base64_decode(b["file"].as_str().unwrap()).unwrap()).unwrap();
    assert!(svg.contains("<svg") && svg.contains("<rect") || svg.contains("<path"), "{svg}");
    assert!(b.get("photos").is_none() && b.as_object().unwrap().len() == 3, "{b}");
    // The draft's page opens; the window says which draft it is.
    assert_eq!(app.ai.publish.made, Some(42));
    assert_eq!(app.pending_url.as_deref(), Some("https://makerfoundry.app/designs/42/edit"));
    assert!(app.ui.status.contains("42"), "{}", app.ui.status);
}

#[test]
fn a_draft_on_another_site_is_not_opened_and_refusals_say_makerfoundrys_sentence() {
    for url in
        ["https://makerfoundry.app.evil.com/designs/1", "https://makerfoundry.app@evil.com/x", "https://evil.com/", "http://makerfoundry.app/d/1"]
    {
        let (mut app, _) = signed_in(vec![(201, json!({"id": 1, "url": url}).to_string())]);
        app.run("ui.publishToMakerFoundry", json!({})).unwrap();
        app.run("ui.publishToMakerFoundry", json!({"send": true})).unwrap();
        wait_published(&mut app);
        assert_eq!(app.pending_url, None, "{url} wasn't opened");
        assert_eq!(app.ai.publish.made, None);
        assert_eq!(app.ai.publish.error.as_deref(), Some("MakerFoundry answered with a page on another site, which wasn't opened."));
    }
    let (mut app, _) = signed_in(vec![(422, json!({"error": {"message": "That isn't a design MakerFoundry can read."}}).to_string())]);
    app.run("ui.publishToMakerFoundry", json!({})).unwrap();
    app.run("ui.publishToMakerFoundry", json!({"send": true})).unwrap();
    wait_published(&mut app);
    assert_eq!(app.ai.publish.error.as_deref(), Some("MakerFoundry: That isn't a design MakerFoundry can read."));
    assert_eq!(app.pending_url, None);
}

#[test]
fn nothing_is_sent_without_a_session_a_design_or_the_window() {
    // Signed out (only an Anthropic key kept): refused before anything leaves.
    let (mut app, sent, _) = app(vec![]);
    app.run("shape.rectangle", json!({"x": 10, "y": 10, "width": 100, "height": 60})).unwrap();
    assert!(app.run("ui.publishToMakerFoundry", json!({"send": true})).unwrap_err().contains("open Publish to MakerFoundry first"));
    let r = app.run("ui.publishToMakerFoundry", json!({})).unwrap();
    assert_eq!(r["signedIn"].as_bool(), Some(false));
    let e = app.run("ui.publishToMakerFoundry", json!({"send": true})).unwrap_err();
    assert!(e.contains("Sign in to your Foundry Account"), "{e}");
    // Signed in, a design ready, but the window shut: nothing goes unseen.
    let (mut seen, sent_seen) = signed_in(vec![]);
    seen.run("ui.publishToMakerFoundry", json!({})).unwrap();
    seen.ai.publish.open = false;
    assert!(seen.run("ui.publishToMakerFoundry", json!({"send": true})).unwrap_err().contains("open Publish to MakerFoundry first"));
    assert!(sent_seen.lock().unwrap().is_empty());
    assert!(sent.lock().unwrap().is_empty());
    // An empty document: nothing to publish, and the command is gray.
    let (mut empty, _, _) = self::app(vec![]);
    assert!(!crate::menus::enabled(&empty, "ui.publishToMakerFoundry"));
    assert!(empty.run("ui.publishToMakerFoundry", json!({})).unwrap_err().contains("nothing to publish"));
}
