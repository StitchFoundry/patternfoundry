//! Start a Pattern (`ui.startPattern`): a pattern starts from measurements, not a page size. One
//! window with three short parts: what to make (bodice, skirt, sleeve or trousers), whose
//! measurements (typed in, a sample size, or a set kept in My Measurements) and, optionally,
//! keeping the typed measurements for next time. Create makes the document (the page is worked out
//! from the drafted pieces afterwards, so nobody picks Letter first), sets the measurements and
//! drafts the block. File › New and the blank-page presets are unchanged.
//!
//! Fields: `block` (`bodice`, `skirt`, `sleeve`, `trousers`), `source` (`enter`, `size`, `saved`),
//! `unit` (`cm` or `in`), `m.<name>` (a typed measurement), `size`, `set` and `save` (keep the
//! typed measurements in My Measurements as `saveName`).
//!
//! The measurements are personal data: they stay in this dialog, the document and, only if the
//! user ticks the box, the folder of saved sets on this computer. Nothing here logs them.

use pf_core::block::{self, BlockKind};
use serde_json::{Map, Value, json};

use super::{DialogSpec, form};
use crate::state::Dialog;
use crate::theme::Tokens;
use crate::{VectorcraftApp, widgets};

pub(super) const KIND: &str = "startPattern";

pub(super) const SPEC: DialogSpec = DialogSpec {
    heading: |_| tl!("Start a Pattern").into(),
    body,
    confirm,
    ok: Some("Create Pattern"),
    min_width: 460.0,
    max_width: Some(520.0),
    ..DialogSpec::FORM
};

/// The blocks to start from: field value, command, kind.
const BLOCKS: [(&str, &str, BlockKind); 4] = [
    ("bodice", "block.bodice", BlockKind::Bodice),
    ("skirt", "block.skirt", BlockKind::Skirt),
    ("sleeve", "block.sleeve", BlockKind::Sleeve),
    ("trousers", "block.trousers", BlockKind::Trousers),
];

/// Sample sizes in centimetres, written for PatternFoundry: round figures to start a pattern
/// from, not a published size standard. One row per size, a column for every measurement the four
/// blocks use.
const SAMPLE_SIZES: &str = "size,bust,waist,hip,nape_to_waist,back_width,chest,shoulder,neck,armscye_depth,waist_to_hip,skirt_length,top_arm,sleeve_length,crotch_depth,waist_to_knee,waist_to_ankle,trouser_hem
XS,82,63,88,40,33,31,11.8,35,20,20,58,26,58,26,57,100,44
S,86,67,92,40.5,34,32,12,36,20.5,20,58,27.5,58.5,26.5,58,101,45
M,92,73,98,41,35,33.5,12.3,37,21,20.5,60,29.5,59,27,59,102,46
L,100,81,106,41.5,36.5,35,12.6,38.5,21.5,21,60,32,59.5,27.5,60,103,47
XL,108,89,114,42,38,37,13,40,22,21.5,61,35,60,28,61,104,48
";
/// The sample sizes' names.
const SAMPLE_NAMES: [&str; 5] = ["XS", "S", "M", "L", "XL"];
/// The page the new document starts with (points); it is fitted to the pieces once they are drafted.
const WORKSPACE_PT: f64 = 3000.0;

/// Open Start a Pattern: a bodice, typed-in measurements, in the unit the preferences use.
pub fn open(app: &mut VectorcraftApp) {
    let inches = app.session.general_unit() == vectorcraft_doc::Unit::Inches;
    let fields = json!({"block": "bodice", "source": "enter", "unit": if inches { "in" } else { "cm" }, "size": "M", "save": false, "saveName": ""});
    app.ui.dialog = Some(Dialog::new(KIND, fields));
}

fn kind_of(d: &Dialog) -> (&'static str, BlockKind) {
    let b = d.str("block");
    BLOCKS.iter().find(|x| x.0 == b).map_or(("block.bodice", BlockKind::Bodice), |x| (x.1, x.2))
}

fn block_label(name: &str) -> &'static str {
    match name {
        "skirt" => tl!("Skirt"),
        "sleeve" => tl!("Sleeve"),
        "trousers" => tl!("Trousers"),
        _ => tl!("Bodice"),
    }
}

fn body(app: &mut VectorcraftApp, ui: &mut egui::Ui, d: &mut Dialog) -> bool {
    let t = Tokens::get(ui.ctx());
    widgets::subheader(ui, tl!("1. What are you making?"));
    ui.horizontal(|ui| {
        for (name, _, _) in BLOCKS {
            if ui.selectable_label(d.str("block") == name, block_label(name)).clicked() {
                d.fields.insert("block".into(), json!(name));
            }
        }
    });
    ui.add_space(10.0);
    widgets::subheader(ui, tl!("2. Whose measurements?"));
    for (source, label) in
        [("enter", tl!("I'll enter my measurements")), ("size", tl!("Pick a sample size")), ("saved", tl!("Use my saved measurements"))]
    {
        if ui.radio(d.str("source") == source, label).clicked() {
            d.fields.insert("source".into(), json!(source));
        }
    }
    ui.add_space(6.0);
    match d.str("source").as_str() {
        "size" => {
            ui.horizontal(|ui| {
                for s in SAMPLE_NAMES {
                    if ui.selectable_label(d.str("size") == s, s).clicked() {
                        d.fields.insert("size".into(), json!(s));
                    }
                }
            });
            widgets::dim_label(ui, tl!("Round sample sizes to start from. Change any measurement afterwards in the Measurements panel."));
        }
        "saved" => {
            let sets: Vec<String> = app.session.measurement_sets().iter().map(|m| m.name.clone()).collect();
            if sets.is_empty() {
                widgets::dim_label(ui, tl!("Nothing saved yet. Enter your measurements once and tick “Keep these measurements”."));
            }
            for name in &sets {
                if ui.selectable_label(d.str("set") == *name, name).clicked() {
                    d.fields.insert("set".into(), json!(name));
                }
            }
        }
        _ => {
            ui.horizontal(|ui| {
                widgets::dim_label(ui, tl!("Units"));
                for (u, label) in [("cm", tl!("centimetres")), ("in", tl!("inches"))] {
                    if ui.selectable_label(d.str("unit") == u, label).clicked() {
                        d.fields.insert("unit".into(), json!(u));
                    }
                }
            });
            let unit = d.str("unit");
            let (_, kind) = kind_of(d);
            for (name, how) in block::measurements(kind) {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(form::humanize(name)).color(t.text));
                    form::text(ui, d, &format!("m.{name}"), 70.0);
                    ui.label(egui::RichText::new(&unit).color(t.text_dim));
                });
                widgets::dim_label(ui, how);
            }
            ui.add_space(6.0);
            form::check(ui, d, "save", tl!("Keep these measurements on this computer (My Measurements)"));
            if d.bool("save") {
                form::field(ui, d, "saveName", tl!("Whose (Me)"));
            }
        }
    }
    false
}

/// The typed measurements as `measure.set` values (`"92 cm"`), or what is missing or wrong.
fn typed(d: &Dialog, kind: BlockKind) -> Result<Map<String, Value>, String> {
    let unit = if d.str("unit") == "in" { "in" } else { "cm" };
    let mut values = Map::new();
    let mut bad = vec![];
    for (name, _) in block::measurements(kind) {
        let text = d.str(&format!("m.{name}")).trim().replace(',', ".");
        match text.parse::<f64>() {
            Ok(v) if v.is_finite() && v > 0.0 && v < 1000.0 => {
                values.insert(name.to_string(), json!(format!("{v} {unit}")));
            }
            _ => bad.push(form::humanize(name)),
        }
    }
    if bad.is_empty() { Ok(values) } else { Err(crate::i18n::fmt(tl!("Enter a number for: {names}"), &[("names", &bad.join(", "))])) }
}

/// Make the document, put the measurements in it, draft the block and fit the page to the pieces.
fn confirm(app: &mut VectorcraftApp, d: &Dialog) -> Result<Value, String> {
    let (command, kind) = kind_of(d);
    let source = d.str("source");
    // Check everything before a document is made, so a mistake leaves nothing behind.
    let values = if source == "enter" { Some(typed(d, kind)?) } else { None };
    let set = d.str("set");
    if source == "saved" && !app.session.measurement_sets().iter().any(|m| m.name == set) {
        return Err(tl!("Choose a saved set of measurements").to_string());
    }
    let units = if d.str("unit") == "in" { "Inches" } else { "Centimeters" };
    app.run("file.new", json!({"width": WORKSPACE_PT, "height": WORKSPACE_PT, "units": units, "name": block_label(&d.str("block"))}))?;
    let made = make(app, d, command, source.as_str(), values, &set);
    if made.is_err() {
        // Nothing was drafted: don't leave an empty page behind.
        app.run("file.close", json!({})).ok();
    } else {
        app.ui.dialog = None;
    }
    made
}

fn make(app: &mut VectorcraftApp, d: &Dialog, command: &str, source: &str, values: Option<Map<String, Value>>, set: &str) -> Result<Value, String> {
    match source {
        "size" => {
            app.run("measure.importChart", json!({"data": SAMPLE_SIZES, "name": "Sample sizes", "unit": "cm"}))?;
            app.run("measure.useSize", json!({"size": d.str("size")}))?;
        }
        "saved" => {
            app.run("measure.loadSet", json!({"name": set}))?;
        }
        _ => {
            app.run("measure.set", json!({"values": values.unwrap_or_default()}))?;
            let name = d.str("saveName");
            if d.bool("save") && !name.trim().is_empty() {
                // Opt-in; a missing folder (the web) is not a reason to lose the pattern.
                app.run("measure.saveSet", json!({"name": name.trim()})).ok();
            }
        }
    }
    let r = app.run(command, json!({}))?;
    app.run("artboard.fitToArt", json!({})).ok();
    Ok(r)
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use vectorcraft_engine::Session;

    use super::*;

    fn pieces(app: &VectorcraftApp) -> usize {
        let mut n = 0;
        app.session.active().unwrap().doc.walk(|node| n += usize::from(node.extras().block.is_some()));
        n
    }

    fn set(app: &mut VectorcraftApp, k: &str, v: Value) {
        app.ui.dialog.as_mut().unwrap().fields.insert(k.into(), v);
    }

    #[test]
    fn a_sample_size_goes_from_launch_to_a_drafted_block_without_a_page_size() {
        let mut app = VectorcraftApp::new(Session::new(), Default::default());
        assert!(app.session.active().is_none());
        app.run("ui.startPattern", json!({})).unwrap();
        set(&mut app, "source", json!("size"));
        set(&mut app, "size", json!("M"));
        crate::dialogs::confirm(&mut app).unwrap();
        assert!(app.ui.dialog.is_none());
        assert_eq!(pieces(&app), 2, "back and front of the bodice");
        let a = app.session.active().unwrap().doc.artboards[0].rect;
        assert!(a.width() < WORKSPACE_PT && a.height() < WORKSPACE_PT, "the page was fitted to the pieces: {a:?}");
    }

    #[test]
    fn typed_measurements_in_inches_draft_the_block_and_can_be_kept() {
        let mut app = VectorcraftApp::new(Session::new(), Default::default());
        let dir = vectorcraft_testkit::temp_dir("start-pattern-keep").join("Measurements");
        let _ = std::fs::remove_dir_all(&dir);
        app.session.attach_measurement_store(dir.clone());
        app.run("ui.startPattern", json!({})).unwrap();
        set(&mut app, "block", json!("skirt"));
        set(&mut app, "unit", json!("in"));
        // Made-up measurements.
        for (k, v) in [("waist", "28"), ("hip", "38"), ("waist_to_hip", "8"), ("skirt_length", "23")] {
            set(&mut app, &format!("m.{k}"), json!(v));
        }
        set(&mut app, "save", json!(true));
        set(&mut app, "saveName", json!("Me"));
        crate::dialogs::confirm(&mut app).unwrap();
        assert_eq!(pieces(&app), 2);
        let m = app.session.active().unwrap().doc.measurements.clone().unwrap();
        assert!((m.values["waist"] - 28.0 * 25.4).abs() < 1e-6, "inches became mm");
        assert_eq!(app.session.measurement_sets().len(), 1, "kept, because the box was ticked");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn missing_measurements_say_which_and_make_no_document() {
        let mut app = VectorcraftApp::new(Session::new(), Default::default());
        app.run("ui.startPattern", json!({})).unwrap();
        set(&mut app, "m.bust", json!("90"));
        let e = crate::dialogs::confirm(&mut app).unwrap_err();
        assert!(e.contains("Waist") && !e.contains("Bust"), "{e}");
        assert!(app.session.active().is_none(), "no empty page left behind");
        assert!(app.ui.dialog.is_some(), "the dialog stays open");
    }

    #[test]
    fn saved_measurements_start_a_pattern_and_nothing_is_kept_unless_ticked() {
        let mut app = VectorcraftApp::new(Session::new(), Default::default());
        let dir = vectorcraft_testkit::temp_dir("start-pattern-saved").join("Measurements");
        let _ = std::fs::remove_dir_all(&dir);
        app.session.attach_measurement_store(dir.clone());
        app.run("file.new", json!({})).unwrap();
        app.run("measure.set", json!({"name": "Me", "values": {"top_arm": "28 cm", "sleeve_length": "59 cm"}})).unwrap();
        app.run("measure.saveSet", json!({})).unwrap();
        app.run("ui.startPattern", json!({})).unwrap();
        set(&mut app, "block", json!("sleeve"));
        set(&mut app, "source", json!("saved"));
        assert!(crate::dialogs::confirm(&mut app).is_err(), "no set chosen yet");
        set(&mut app, "set", json!("Me"));
        crate::dialogs::confirm(&mut app).unwrap();
        assert_eq!(pieces(&app), 1);
        assert_eq!(app.session.measurement_sets().len(), 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_blank_page_path_still_works() {
        let mut app = VectorcraftApp::new(Session::new(), Default::default());
        app.run("file.newDialog", json!({})).unwrap();
        assert_eq!(app.ui.dialog.as_ref().unwrap().kind, "newDocument");
        crate::dialogs::confirm(&mut app).unwrap();
        assert!(app.session.active().is_some());
        assert_eq!(pieces(&app), 0);
    }

    #[test]
    fn the_window_draws_each_source_without_a_page_size_field() {
        for source in ["enter", "size", "saved"] {
            let mut app = VectorcraftApp::new(Session::new(), Default::default());
            open(&mut app);
            set(&mut app, "source", json!(source));
            let text = crate::tests_labels::painted_text(&mut app, |app, ui| crate::dialogs::show(app, ui.ctx()));
            assert!(text.contains("What are you making?") && text.contains("Whose measurements?"), "{source}: {text}");
            assert!(!text.contains("Letter") && !text.contains("artboard") && !text.contains("bleed"), "{source}: plain words only");
        }
    }
}
