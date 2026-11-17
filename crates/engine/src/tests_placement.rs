//! Designs placed on a piece (PF-12.1): `piece.addPlacement`, `setPlacement`, `removePlacement`.

use pf_core::mm_to_pt;
use serde_json::{Value, json};
use vectorcraft_doc::NodeId;

use super::*;

/// A session with a 10 cm square made into a piece, selected.
fn piece() -> (Session, NodeId) {
    let mut s = Session::new();
    s.execute("file.new", &json!({"width": 800, "height": 800})).unwrap();
    let side = mm_to_pt(100.0);
    let r = s.execute("shape.rectangle", &json!({"x": 0, "y": 0, "width": side, "height": side})).unwrap();
    let id = r["id"].as_u64().unwrap();
    let made = s.execute("piece.make", &json!({"ids": [id]})).unwrap();
    (s, NodeId(made["ids"][0].as_u64().unwrap()))
}

fn rose(id: NodeId) -> Value {
    json!({"id": id.0, "name": "Rose", "widthMm": 90, "heightMm": 60, "at": [100, 100], "hoop": [100, 100]})
}

#[test]
fn a_design_is_placed_at_its_true_size() {
    let (mut s, id) = piece();
    let info = s.execute("piece.addPlacement", &rose(id)).unwrap();
    let d = &info["placements"][0];
    assert_eq!(d["name"], "Rose");
    assert_eq!(d["kind"], "embroidery");
    assert_eq!(d["width"], 90.0);
    assert_eq!(d["scale"], 1.0);
    assert_eq!(d["hoop"]["width"], 100.0);
}

#[test]
fn it_moves_scales_to_a_true_width_turns_and_loses_its_hoop() {
    let (mut s, id) = piece();
    s.execute("piece.addPlacement", &rose(id)).unwrap();
    let info =
        s.execute("piece.setPlacement", &json!({"id": id.0, "index": 0, "at": [10, 20], "width": "45 mm", "rotation": -90, "hoop": null})).unwrap();
    let d = &info["placements"][0];
    assert_eq!(d["at"], json!({"x": 10.0, "y": 20.0}));
    assert!((d["scale"].as_f64().unwrap() - 0.5).abs() < 1e-12);
    assert_eq!(d["rotation"], 270.0);
    assert!(d.get("hoop").is_none());
}

#[test]
fn removing_and_undoing_a_placement() {
    let (mut s, id) = piece();
    s.execute("piece.addPlacement", &json!({"id": id.0, "name": "Logo", "widthMm": 40, "heightMm": 40, "at": [0, 0], "kind": "screen"})).unwrap();
    let info = s.execute("piece.removePlacement", &json!({"id": id.0, "index": 0})).unwrap();
    assert!(info.get("placements").is_none());
    s.execute("edit.undo", &json!({})).unwrap();
    let info = s.execute("piece.info", &json!({"id": id.0})).unwrap();
    assert_eq!(info["placements"][0]["kind"], "screen");
}

#[test]
fn bad_placements_are_refused_and_change_nothing() {
    let (mut s, id) = piece();
    let before = s.doc().unwrap().history.undo.len();
    let mut cases = vec![
        json!({"name": "x", "widthMm": 0, "heightMm": 10, "at": [0, 0]}),
        json!({"name": "x", "widthMm": 10, "heightMm": 10}),
        json!({"name": "x", "widthMm": 10, "heightMm": 10, "at": [0, 0], "scale": 0}),
        json!({"name": "x", "widthMm": 10, "heightMm": 10, "at": [0, 0], "kind": "laser"}),
        json!({"name": "x", "widthMm": 10, "heightMm": 10, "at": [0, 0], "hoop": [10]}),
        json!({"widthMm": 10, "heightMm": 10, "at": [0, 0]}),
    ];
    for c in &mut cases {
        c["id"] = json!(id.0);
        assert!(s.execute("piece.addPlacement", c).is_err(), "{c}");
    }
    for cmd in ["piece.setPlacement", "piece.removePlacement"] {
        assert!(s.execute(cmd, &json!({"id": id.0, "index": 0})).is_err(), "{cmd}");
    }
    assert_eq!(s.doc().unwrap().history.undo.len(), before);
}

/// A 10 cm square piece at 40 pt with a 10 mm allowance, a 40 x 20 mm design at its centre.
fn placed_for_export() -> Session {
    let mut s = Session::new();
    s.execute("file.new", &json!({"width": 800, "height": 800})).unwrap();
    s.execute("shape.rectangle", &json!({"x": 40, "y": 40, "width": mm_to_pt(100.0), "height": mm_to_pt(100.0)})).unwrap();
    let made = s.execute("piece.make", &json!({"name": "Front", "allowance": 10})).unwrap();
    let id = made["ids"][0].as_u64().unwrap();
    let c = 40.0 + mm_to_pt(50.0);
    s.execute("piece.addPlacement", &json!({"id": id, "name": "Rose", "widthMm": 40, "heightMm": 20, "at": [c, c]})).unwrap();
    s
}

#[test]
fn the_cutting_svg_draws_the_design_true_size_with_its_crosshair_and_note() {
    let mut s = placed_for_export();
    let r = s.execute("document.export", &json!({"format": "svgcut"})).unwrap();
    let svg = String::from_utf8(vectorcraft_format::base64_decode(r["dataBase64"].as_str().unwrap()).unwrap()).unwrap();
    // Page mm: the seam line starts 20 mm in, so the 40 x 20 mm outline centred at 70, 70 is exact.
    assert!(svg.contains("M50 60 L90 60 L90 80 L50 80 L50 60"), "{svg}");
    assert!(svg.contains("M65 70 L75 70") && svg.contains("M70 65 L70 75"), "{svg}");
    assert!(svg.contains("Embroider &quot;Rose&quot; here, 40 x 20 mm"), "{svg}");
}

#[test]
fn the_link_names_the_app_and_checks_the_file() {
    let (mut s, id) = piece();
    let dir = std::env::temp_dir().join(format!("pf_link_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let dst = dir.join("rose.DST");
    std::fs::write(&dst, b"x").unwrap();
    let mut a = rose(id);
    a["file"] = json!(dst.to_str().unwrap());
    s.execute("piece.addPlacement", &a).unwrap();
    s.execute(
        "piece.addPlacement",
        &json!({"id": id.0, "name": "Logo", "widthMm": 40, "heightMm": 40, "at": [0, 0], "kind": "screen", "file": "/no/such/logo.png"}),
    )
    .unwrap();
    let r = s.execute("piece.placementLink", &json!({"id": id.0})).unwrap();
    assert_eq!(r["links"][0]["app"], "StitchFoundry");
    assert_eq!(r["links"][0]["openable"], true);
    assert_eq!(r["links"][1]["app"], "ScreenFoundry");
    assert_eq!(r["links"][1]["openable"], false);
    let one = s.execute("piece.placementLink", &json!({"id": id.0, "index": 1})).unwrap();
    assert_eq!(one["links"].as_array().unwrap().len(), 1);
    assert!(s.execute("piece.placementLink", &json!({"id": id.0, "index": 2})).is_err());
    std::fs::remove_dir_all(&dir).ok();
}
