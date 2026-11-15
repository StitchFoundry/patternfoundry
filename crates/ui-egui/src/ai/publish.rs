//! File › Publish to MakerFoundry… (PF-11): the document as SVG, sent with the person's Foundry
//! Account session to MakerFoundry as a private draft ([`pf_ai::makerfoundry`]), whose page then
//! opens in the browser for them to finish. The window shows what goes and where before anything
//! is sent; Publish sends it.

use std::sync::mpsc::{Receiver, TryRecvError};

use egui::RichText;
use pf_ai::makerfoundry::{self, MfError, Published};
use serde_json::{Value, json};

use crate::VectorcraftApp;
use crate::theme::Tokens;

/// The design ready to go: its size and contents (base64).
#[derive(Clone)]
struct Design {
    len: usize,
    base64: String,
}

/// The window between frames.
#[derive(Default)]
pub struct PublishState {
    pub open: bool,
    /// The design's name as typed (its file name and so the draft's title on MakerFoundry: see
    /// [`makerfoundry::file_name`]); the document's title to begin with.
    pub name: String,
    design: Option<Design>,
    /// The answer on its way: the network's error, or MakerFoundry's answer read.
    pending: Option<Receiver<Result<Result<Published, MfError>, pf_ai::AiError>>>,
    pub error: Option<String>,
    /// The draft made (its id), shown once it is.
    pub made: Option<u64>,
}

/// What the app says it is: "PatternFoundry 0.13.1".
fn app_name() -> String {
    format!("PatternFoundry {}", env!("CARGO_PKG_VERSION"))
}

/// The active document as the SVG file MakerFoundry gets.
fn prepare(app: &mut VectorcraftApp) -> Result<Design, String> {
    let st = app.session.active().ok_or("no document open")?;
    if st.doc.art_bounds().is_none() {
        return Err(MfError::Empty.to_string());
    }
    let v = app.session.execute("document.serialize", &json!({ "format": "svg" })).map_err(|e| e.to_string())?;
    let svg = v["text"].as_str().filter(|s| !s.is_empty()).ok_or_else(|| MfError::Empty.to_string())?;
    if svg.len() > makerfoundry::MAX_FILE {
        return Err(MfError::TooLarge.to_string());
    }
    Ok(Design { len: svg.len(), base64: vectorcraft_format::base64_encode(svg.as_bytes()) })
}

/// The document's title as a name to start from: its file name without the folder or extension.
fn start_name(title: &str) -> String {
    let stem = title.rsplit(['/', '\\']).next().unwrap_or_default();
    let stem = stem.rsplit_once('.').map_or(stem, |(s, _)| s);
    stem.trim().to_string()
}

/// `ui.publishToMakerFoundry`: `{name?}` opens the window with the document ready to go
/// (nothing is sent); `{send: true, name?}` sends what the open window shows.
pub fn command(app: &mut VectorcraftApp, p: &Value) -> Result<Value, String> {
    if !super::available(app) {
        return Err("Publish to MakerFoundry needs a build that can reach MakerFoundry".into());
    }
    super::account::load(app);
    let named = p.get("name").and_then(Value::as_str).map(|n| n.trim().chars().take(200).collect::<String>());
    if p.get("send").and_then(Value::as_bool) == Some(true) {
        if let Some(n) = named.filter(|_| app.ai.publish.open) {
            app.ai.publish.name = n;
        }
        if !app.ai.publish.open || app.ai.publish.design.is_none() {
            return Err("open Publish to MakerFoundry first: it shows what is sent".into());
        }
        send(app);
        let st = &app.ai.publish;
        return match &st.error {
            Some(e) => Err(e.clone()),
            None => Ok(json!({ "sending": st.pending.is_some() })),
        };
    }
    let design = prepare(app)?;
    let title = app.session.active().map(|st| st.title()).unwrap_or_default();
    let st = &mut app.ai.publish;
    st.open = true;
    st.error = None;
    st.made = None;
    st.name = named.unwrap_or_else(|| start_name(&title));
    let out = json!({
        "open": true,
        "name": st.name,
        "fileName": makerfoundry::file_name(&st.name, "svg"),
        "bytes": design.len,
        "sentTo": format!("{}{}", makerfoundry::site(), makerfoundry::DESIGNS),
        "signedIn": !app.ai.account.token.is_empty(),
    });
    app.ai.publish.design = Some(design);
    Ok(out)
}

/// Send the design the window shows with the person's session.
fn send(app: &mut VectorcraftApp) {
    let st = &mut app.ai.publish;
    st.error = None;
    if st.pending.is_some() {
        return;
    }
    let Some(d) = st.design.clone() else { return };
    let file_name = makerfoundry::file_name(&st.name, "svg");
    let http = match makerfoundry::publish(&app.ai.account.token, &file_name, &d.base64, d.len, &app_name()) {
        Ok(h) => h,
        Err(e) => {
            app.ai.publish.error = Some(e.to_string());
            return;
        }
    };
    match super::post_plain(app, http, "makerfoundry-publish", |status, body| Ok(makerfoundry::published(status, body))) {
        Ok(rx) => app.ai.publish.pending = Some(rx),
        Err(e) => app.ai.publish.error = Some(e),
    }
}

/// Each frame: take in MakerFoundry's answer; a draft opens in the browser.
pub fn poll(app: &mut VectorcraftApp, ctx: &egui::Context) {
    let Some(rx) = app.ai.publish.pending.as_ref() else { return };
    let r = match rx.try_recv() {
        Ok(r) => r.map_err(|e| MfError::Network(e.to_string())).and_then(|r| r),
        Err(TryRecvError::Empty) => {
            ctx.request_repaint_after(std::time::Duration::from_millis(250));
            return;
        }
        Err(TryRecvError::Disconnected) => Err(MfError::Network("the request stopped".into())),
    };
    app.ai.publish.pending = None;
    answered(app, r);
}

fn answered(app: &mut VectorcraftApp, r: Result<Published, MfError>) {
    match r {
        // `published` only gives pages on MakerFoundry's own site.
        Ok(p) => {
            app.open_url(&p.url);
            let st = &mut app.ai.publish;
            st.made = Some(p.id);
            st.design = None;
            app.status(crate::i18n::fmt(
                tl!("Draft {id} made on MakerFoundry: finish it on the page that opened in your browser."),
                &[("id", &p.id.to_string())],
            ));
        }
        Err(e) => app.ai.publish.error = Some(e.to_string()),
    }
}

pub fn window(app: &mut VectorcraftApp, ctx: &egui::Context) {
    if !app.ai.publish.open {
        return;
    }
    let t = Tokens::get(ctx);
    let mut open = true;
    let (mut publish, mut sign_in) = (false, false);
    egui::Window::new(tl!("Publish to MakerFoundry"))
        .id(egui::Id::new("publish-makerfoundry"))
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .default_pos(ctx.content_rect().center() - egui::vec2(220.0, 140.0))
        .show(ctx, |ui| {
            ui.set_width(440.0);
            let signed_in = !app.ai.account.token.is_empty();
            let st = &mut app.ai.publish;
            if let Some(id) = st.made {
                ui.label(RichText::new(crate::i18n::fmt(tl!("Draft {id} made."), &[("id", &id.to_string())])).strong());
                ui.label(tl!("Finish it on MakerFoundry's page, which opened in your browser: it stays private until you publish it there."));
            } else if let Some(d) = &st.design {
                ui.label(tl!(
                    "Your design goes to MakerFoundry as a private draft, to finish on its page. Nothing is public until you publish it there."
                ));
                ui.add_space(6.0);
                let len = d.len;
                egui::Grid::new("publish-what").num_columns(2).spacing([10.0, 4.0]).show(ui, |ui| {
                    let name_label = ui.label(tl!("Name"));
                    // (labelled for assistive technology and test drivers: the field's accessible name is "Name")
                    ui.add(egui::TextEdit::singleline(&mut st.name).desired_width(280.0).hint_text(tl!("What it is, e.g. A-line skirt")))
                        .labelled_by(name_label.id);
                    ui.end_row();
                    ui.label(tl!("File"));
                    ui.label(format!("{}  ({:.1} KB)", makerfoundry::file_name(&st.name, "svg"), len as f64 / 1024.0));
                    ui.end_row();
                    ui.label(tl!("Kind"));
                    ui.label(tl!("SVG, a vector design"));
                    ui.end_row();
                    ui.label(tl!("Sent to"));
                    ui.label(format!("{}{}", makerfoundry::site(), makerfoundry::DESIGNS));
                    ui.end_row();
                    ui.label(tl!("Account"));
                    let who = app.ai.account.info.as_ref().and_then(|i| i["email"].as_str()).map(str::to_string);
                    ui.label(match (signed_in, who) {
                        (true, Some(email)) => email,
                        (true, None) => tl!("Your Foundry Account").to_string(),
                        (false, _) => tl!("Not signed in").to_string(),
                    });
                    ui.end_row();
                });
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if signed_in {
                        let busy = st.pending.is_some();
                        if ui.add_enabled_ui(!busy, |ui| crate::widgets::primary_button(ui, tl!("Publish"))).inner.clicked() {
                            publish = true;
                        }
                        if busy {
                            ui.spinner();
                        }
                    } else if crate::widgets::primary_button(ui, tl!("Sign In…")).clicked() {
                        sign_in = true;
                    }
                });
            }
            if let Some(e) = &app.ai.publish.error {
                ui.label(RichText::new(crate::i18n::msg(e)).color(t.error));
            }
        });
    if publish {
        send(app);
    }
    if sign_in {
        let _ = super::account::open(app, &json!({ "create": false }));
    }
    if !open {
        app.ai.publish = PublishState::default();
    }
}
