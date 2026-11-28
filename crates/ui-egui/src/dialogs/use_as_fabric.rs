//! Pattern › Use as Fabric… (`ui.useAsFabric`, PF-10.16): how wide the selected photo is on the
//! fabric (cm or inches), which way its lengthwise grain runs, even out the light, make it repeat.
//! OK runs `fabric.set`, then says the drape needs drawing again to show it.
//!
//! Fields: `__id` (the image), `width` (mm), `inches` (the width is shown in inches), `name`,
//! `turns` (0 to 3), `even` and `seamless`.

use serde_json::{Value, json};

use super::DialogSpec;
use crate::state::Dialog;
use crate::{VectorcraftApp, widgets};

/// The dialog kind of Use as Fabric.
pub const KIND: &str = "useAsFabric";

/// Width of the label column.
const LABEL_W: f32 = 150.0;

/// Quarter-turn buttons: what the lengthwise grain is turned by, and its name.
const TURNS: [&str; 4] = ["0°", "90°", "180°", "270°"];

pub(super) const SPEC: DialogSpec = DialogSpec { heading: |_| tl!("Use as Fabric").into(), body, confirm, min_width: 400.0, ..DialogSpec::FORM };

/// Open Use as Fabric on the selected image (or the image `id` in `p`), starting from what the
/// photo is now (`fabric.info`): its width on the fabric, else as wide as it is placed.
pub fn open(app: &mut VectorcraftApp, p: &Value) -> Result<Value, String> {
    let params = match p.get("id") {
        Some(id) => json!({ "id": id }),
        None => json!({}),
    };
    let info = app.session.execute("fabric.info", &params).map_err(|e| e.to_string())?;
    let inches = app.session.general_unit() == vectorcraft_doc::Unit::Inches;
    let fields = json!({
        "__id": info["id"], "width": info["width"], "inches": inches, "name": info["name"],
        "turns": info["turns"], "even": info["even"], "seamless": info["seamless"],
    });
    app.ui.dialog = Some(Dialog::new(KIND, fields));
    Ok(info)
}

fn body(_app: &mut VectorcraftApp, ui: &mut egui::Ui, d: &mut Dialog) -> bool {
    ui.label("How wide is the fabric in this photo?");
    let inches = d.fields.get("inches").and_then(Value::as_bool).unwrap_or(false);
    let per = if inches { 25.4 } else { 10.0 };
    widgets::label_row(ui, "Width", LABEL_W, |ui| {
        let mut w = d.f64("width", 1000.0) / per;
        if ui.add(egui::DragValue::new(&mut w).range(1.0..=5000.0 / per).speed(0.5).max_decimals(1)).changed() {
            d.fields.insert("width".into(), json!(w * per));
        }
        for (label, is_in) in [("cm", false), ("in", true)] {
            if ui.selectable_label(inches == is_in, label).clicked() {
                d.fields.insert("inches".into(), json!(is_in));
            }
        }
    });
    widgets::label_row(ui, "Fabric", LABEL_W, |ui| {
        let mut name = d.fields.get("name").and_then(Value::as_str).unwrap_or_default().to_string();
        if ui.add(egui::TextEdit::singleline(&mut name).hint_text("the main fabric").desired_width(180.0)).changed() {
            d.fields.insert("name".into(), json!(name));
        }
    });
    widgets::label_row(ui, "Lengthwise grain turned", LABEL_W, |ui| {
        let turns = d.fields.get("turns").and_then(Value::as_u64).unwrap_or(0);
        for (i, label) in TURNS.iter().enumerate() {
            if ui.selectable_label(turns == i as u64, *label).on_hover_text("Quarter turns clockwise that bring the selvedge upright in the photo").clicked() {
                d.fields.insert("turns".into(), json!(i));
            }
        }
    });
    for (key, label) in [("even", "Even out light"), ("seamless", "Make repeat")] {
        let mut on = d.fields.get(key).and_then(Value::as_bool).unwrap_or(false);
        if ui.checkbox(&mut on, label).changed() {
            d.fields.insert(key.into(), json!(on));
        }
    }
    false
}

fn confirm(app: &mut VectorcraftApp, d: &Dialog) -> Result<Value, String> {
    let f = |k: &str| d.fields.get(k).cloned().unwrap_or(Value::Null);
    let params = json!({
        "id": f("__id"), "width": f("width"), "name": f("name"), "turns": f("turns"), "even": f("even"), "seamless": f("seamless"),
    });
    let r = app.session.execute("fabric.set", &params).map_err(|e| e.to_string())?;
    app.ui.dialog = None;
    crate::dialogs::confirm::tell(app, "Fabric set", "Drape on Avatar again to see it.");
    Ok(r)
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use vectorcraft_engine::Session;

    use crate::VectorcraftApp;

    #[test]
    fn use_as_fabric_asks_for_the_width_and_says_to_drape_again() {
        let mut app = VectorcraftApp::new(Session::new(), Default::default());
        app.run("file.new", json!({"width": 1400, "height": 1000})).unwrap();
        assert!(app.run("ui.useAsFabric", json!({})).is_err(), "no image selected");
        let mut png = Vec::new();
        image::RgbaImage::from_pixel(64, 32, image::Rgba([200, 30, 30, 255])).write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png).unwrap();
        let placed = app.run("file.place", json!({"name": "wool.png", "dataBase64": vectorcraft_format::base64_encode(&png)})).unwrap();
        let id = placed["ids"][0].as_u64().unwrap();
        app.run("select.set", json!({"ids": [id]})).unwrap();
        app.run("ui.useAsFabric", json!({})).unwrap();
        let d = app.ui.dialog.as_ref().unwrap();
        assert_eq!(d.kind, "useAsFabric");
        let placed = d.fields["width"].as_f64().unwrap();
        assert!(placed > 0.0, "the width starts from the placed picture");
        super::super::tests_export::frame(&mut app);
        let d = app.ui.dialog.as_mut().unwrap();
        d.fields.insert("width".into(), json!(1500.0));
        d.fields.insert("turns".into(), json!(1));
        d.fields.insert("even".into(), json!(true));
        d.fields.insert("seamless".into(), json!(true));
        super::super::confirm(&mut app).unwrap();
        let fabrics = app.run("fabric.list", json!({})).unwrap();
        assert_eq!(fabrics["fabrics"][0]["width"], json!(1500.0));
        assert_eq!(fabrics["fabrics"][0]["turns"], json!(1));
        assert_eq!(fabrics["fabrics"][0]["even"], json!(true));
        assert_eq!(fabrics["fabrics"][0]["seamless"], json!(true));
        let told = app.ui.dialog.as_ref().unwrap();
        assert_eq!(told.fields["message"], json!("Fabric set"), "the user is told to drape again");
    }
}
