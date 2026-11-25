//! The measurements Draw Avatar and Drape on Avatar need and the document lacks: a small window
//! listing just the missing ones, each with how to measure it, in the unit the user chose. OK
//! puts them in the document (`measure.set`) and runs the command again.
//!
//! Fields: `__command` and `__params` (what to run after), `unit` (`cm` or `in`) and `m.<name>`
//! (a typed measurement). The measurements are personal data: nothing here logs them.

use serde_json::{Map, Value, json};

use super::{DialogSpec, form};
use crate::state::Dialog;
use crate::theme::Tokens;
use crate::{VectorcraftApp, widgets};

pub(super) const KIND: &str = "neededMeasurements";

pub(super) const SPEC: DialogSpec = DialogSpec {
    heading: |_| tl!("Measurements Needed").into(),
    body,
    confirm,
    ok: Some("OK"),
    min_width: 420.0,
    max_width: Some(480.0),
    ..DialogSpec::FORM
};

/// The commands that draw the avatar.
const COMMANDS: [&str; 2] = ["avatar.draw", "drape.make"];

/// The avatar's measurements the open document lacks: (name, how to measure it).
fn missing(app: &VectorcraftApp) -> Vec<(&'static str, &'static str)> {
    let Some(st) = app.session.active() else { return vec![] };
    let mut env = st.doc.measurements.as_deref().map(pf_core::measure::Measurements::env).unwrap_or_default();
    pf_core::avatar::MEASUREMENTS.iter().copied().filter(|(name, _)| !env.value(name).is_ok_and(|v| v > 0.0)).collect()
}

/// A measurement's name as a label: "nape_to_waist" → "Nape to waist".
fn label(name: &str) -> String {
    let s = name.replace('_', " ");
    let mut c = s.chars();
    c.next().map(|f| f.to_uppercase().chain(c).collect()).unwrap_or_default()
}

/// If `id` draws the avatar and the document lacks measurements for it, open the window that asks
/// for them and return what to tell the user; None when nothing is missing.
pub fn ask(app: &mut VectorcraftApp, id: &str, params: &Value) -> Option<String> {
    if !COMMANDS.contains(&id) {
        return None;
    }
    let lacking = missing(app);
    if lacking.is_empty() {
        return None;
    }
    let inches = app.session.general_unit() == vectorcraft_doc::Unit::Inches;
    let fields = json!({"__command": id, "__params": params, "unit": if inches { "in" } else { "cm" }});
    app.ui.dialog = Some(Dialog::new(KIND, fields));
    let names: Vec<String> = lacking.iter().map(|(n, _)| label(n)).collect();
    Some(crate::i18n::fmt(tl!("The avatar needs your measurements: {names}."), &[("names", &names.join(", "))]))
}

fn body(app: &mut VectorcraftApp, ui: &mut egui::Ui, d: &mut Dialog) -> bool {
    let t = Tokens::get(ui.ctx());
    widgets::dim_label(ui, tl!("The avatar is made from these measurements. Enter the ones it is missing."));
    ui.horizontal(|ui| {
        widgets::dim_label(ui, tl!("Units"));
        for (u, label) in [("cm", tl!("centimetres")), ("in", tl!("inches"))] {
            if ui.selectable_label(d.str("unit") == u, label).clicked() {
                d.fields.insert("unit".into(), json!(u));
            }
        }
    });
    let unit = d.str("unit");
    for (name, how) in missing(app) {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(label(name)).color(t.text));
            form::text(ui, d, &format!("m.{name}"), 70.0);
            ui.label(egui::RichText::new(&unit).color(t.text_dim));
        });
        widgets::dim_label(ui, how);
    }
    false
}

/// Put the typed measurements in the document and run the command again.
fn confirm(app: &mut VectorcraftApp, d: &Dialog) -> Result<Value, String> {
    let unit = if d.str("unit") == "in" { "in" } else { "cm" };
    let mut values = Map::new();
    let mut bad = vec![];
    for (name, _) in missing(app) {
        let text = d.str(&format!("m.{name}")).trim().replace(',', ".");
        match text.parse::<f64>() {
            Ok(v) if v.is_finite() && v > 0.0 && v < 1000.0 => {
                values.insert(name.to_string(), json!(format!("{v} {unit}")));
            }
            _ => bad.push(label(name)),
        }
    }
    if !bad.is_empty() {
        return Err(crate::i18n::fmt(tl!("Enter a number for: {names}"), &[("names", &bad.join(", "))]));
    }
    let params = d.fields.get("__params").cloned().unwrap_or_else(|| json!({}));
    app.run("measure.set", json!({"values": values}))?;
    app.ui.dialog = None;
    app.run(&d.str("__command"), params)
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use vectorcraft_engine::Session;

    use super::*;

    #[test]
    fn a_skirt_only_user_is_asked_for_bust_and_nape_to_waist_then_the_avatar_draws() {
        let mut app = VectorcraftApp::new(Session::new(), Default::default());
        app.run("file.new", json!({"width": 1400, "height": 1000})).unwrap();
        // Made-up skirt measurements, as Start a Pattern › Skirt sets them.
        app.run("measure.set", json!({"values": {"waist": 680, "hip": 960, "waist_to_hip": 200, "skirt_length": 600}})).unwrap();
        let e = app.run("avatar.draw", json!({"origin": [0, 0]})).unwrap_err();
        assert!(!e.contains("measure.set") && !e.contains("mm,"), "plain words: {e}");
        assert!(e.contains("Bust") && e.contains("Nape to waist") && !e.contains("Waist to hip"), "{e}");
        assert_eq!(app.ui.dialog.as_ref().unwrap().kind, KIND);
        let text = crate::tests_labels::painted_text(&mut app, |app, ui| crate::dialogs::show(app, ui.ctx()));
        assert!(text.contains("Bust") && text.contains("Nape to waist") && !text.contains("Hip"), "{text}");
        // Nothing typed yet: it says which, and stays open.
        assert!(crate::dialogs::confirm(&mut app).is_err());
        assert!(app.ui.dialog.is_some());
        let mut set = |k: &str, v: &str| app.ui.dialog.as_mut().unwrap().fields.insert(k.into(), json!(v));
        set("unit", "in");
        set("m.bust", "34.5");
        set("m.nape_to_waist", "16");
        crate::dialogs::confirm(&mut app).unwrap();
        assert!(app.ui.dialog.is_none());
        let m = app.session.active().unwrap().doc.measurements.clone().unwrap();
        assert!((m.values["bust"] - 34.5 * 25.4).abs() < 1e-6, "inches became mm");
        assert!(app.session.active().unwrap().doc.layers.iter().any(|l| l.name.as_deref() == Some("Avatar")), "the avatar is drawn");
    }
}
