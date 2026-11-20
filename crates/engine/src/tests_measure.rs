//! Measurements and drafting (PF-3, `measure.*`, `draft.*`): a draft follows the measurements,
//! and personal measurements stay out of the files made to share.

use pf_core::mm_to_pt;
use serde_json::{Value, json};
use vectorcraft_doc::{NodeId, NodeKind};

use super::*;

fn session() -> Session {
    let mut s = Session::new();
    s.execute("file.new", &json!({"width": 1000, "height": 1000})).unwrap();
    s
}

/// A bodice block outline: centre front (0, 0) to the side at the bust line, down to the waist and
/// back. Anchors: 0 neck, 1 side at the bust line, 2 side at the waist, 3 centre front at the waist.
fn bodice(s: &mut Session) -> NodeId {
    let r = s
        .execute("path.create", &json!({"anchors": [{"x": 0, "y": 0}, {"x": 200, "y": 0}, {"x": 200, "y": 300}, {"x": 0, "y": 300}], "closed": true}))
        .unwrap();
    NodeId(r["id"].as_u64().unwrap())
}

fn anchor(s: &Session, id: NodeId, i: usize) -> (f64, f64) {
    let n = s.doc().unwrap().doc.node(id).unwrap().clone();
    let NodeKind::Path { path, .. } = &n.kind else { panic!("not a path") };
    let a = path.subpaths[0].anchors[i].p;
    (a.x, a.y)
}

fn near(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-6
}

#[test]
fn changing_the_bust_redraws_the_bodice() {
    let mut s = session();
    let path = bodice(&mut s);
    s.execute(
        "measure.set",
        &json!({"name": "Me", "values": {"bust": 920, "back_length": "40 cm"}, "formulas": {"ease": "40", "quarter": "(bust + ease) / 4"}}),
    )
    .unwrap();
    let info = s.execute("measure.info", &json!({})).unwrap();
    assert_eq!(info["computed"]["quarter"], json!(240.0));
    assert_eq!(info["computed"]["back_length"], json!(400.0));
    // The side seam's two points sit a quarter of the bust (with ease) from centre front; the
    // waist a back length down.
    s.execute("select.set", &json!({"ids": [path.0]})).unwrap();
    s.execute("draft.setPoint", &json!({"anchor": 1, "x": "quarter", "origin": [0, 0]})).unwrap();
    s.execute("draft.setPoint", &json!({"anchor": 2, "x": "quarter", "y": "back_length"})).unwrap();
    s.execute("draft.setPoint", &json!({"anchor": 3, "y": "back_length"})).unwrap();
    assert!(near(anchor(&s, path, 1).0, mm_to_pt(240.0)) && near(anchor(&s, path, 2).1, mm_to_pt(400.0)));
    // Make it a pattern piece, then change the bust: the side seam moves out, and the cutting
    // line with it.
    let g = NodeId(s.execute("piece.make", &json!({})).unwrap()["ids"][0].as_u64().unwrap());
    let cut_right = |s: &Session| {
        let group = s.doc().unwrap().doc.node(g).unwrap().clone();
        let cut = group
            .children()
            .unwrap()
            .iter()
            .find(|c| c.extras().pattern.as_deref().and_then(pf_core::PatternTag::mark) == Some(pf_core::Mark::Cut))
            .unwrap()
            .clone();
        let NodeKind::Path { path, .. } = &cut.kind else { panic!() };
        path.bounds().unwrap().x1
    };
    assert!(near(cut_right(&s), mm_to_pt(240.0 + 10.0)));
    s.execute("measure.set", &json!({"values": {"bust": 1000}})).unwrap();
    assert!(near(anchor(&s, path, 1).0, mm_to_pt(260.0)), "{:?}", anchor(&s, path, 1));
    assert!(near(cut_right(&s), mm_to_pt(260.0 + 10.0)));
    // One undo step: back to the old bust, side seam and cutting line.
    s.execute("edit.undo", &json!({})).unwrap();
    assert!(near(anchor(&s, path, 1).0, mm_to_pt(240.0)));
    assert!(near(cut_right(&s), mm_to_pt(250.0)));
    // A point without a formula on an axis keeps it: anchor 1's y stays where it was drawn.
    assert!(near(anchor(&s, path, 1).1, 0.0));
    let d = s.execute("draft.info", &json!({"id": path.0})).unwrap();
    assert_eq!(d["points"].as_array().unwrap().len(), 3);
}

#[test]
fn moving_a_drafted_path_moves_its_origin_and_its_points_stay_put_relative_to_it() {
    let mut s = session();
    let path = bodice(&mut s);
    s.execute("measure.set", &json!({"values": {"w": 100}})).unwrap();
    s.execute("select.set", &json!({"ids": [path.0]})).unwrap();
    s.execute("draft.setPoint", &json!({"anchor": 1, "x": "w", "origin": [0, 0]})).unwrap();
    s.execute("object.move", &json!({"dx": 50, "dy": 20})).unwrap();
    assert!(near(anchor(&s, path, 1).0, 50.0 + mm_to_pt(100.0)), "{:?}", anchor(&s, path, 1));
    let d = s.execute("draft.info", &json!({})).unwrap();
    assert_eq!(d["origin"], json!([50.0, 20.0]));
}

#[test]
fn a_size_chart_fills_in_the_measurements() {
    let mut s = session();
    s.execute("measure.importChart", &json!({"name": "Test", "data": "size,bust,waist\n10,86,68\n12,91,73\n"})).unwrap();
    s.execute("measure.set", &json!({"formulas": {"ease": "40"}})).unwrap();
    let info = s.execute("measure.useSize", &json!({"size": "12"})).unwrap();
    assert_eq!(info["values"], json!({"bust": 910.0, "waist": 730.0}));
    assert_eq!(info["size"], json!("Test 12"));
    assert_eq!(info["chart"]["sizes"], json!(["10", "12"]));
    assert!(s.execute("measure.useSize", &json!({"size": "99"})).is_err());
    // Measuring by hand takes it off the chart's size.
    let info = s.execute("measure.set", &json!({"values": {"bust": 900}})).unwrap();
    assert!(info["size"].is_null());
}

#[test]
fn bad_measurements_and_formulas_are_refused_without_changing_anything() {
    let mut s = session();
    let path = bodice(&mut s);
    s.execute("measure.set", &json!({"values": {"bust": 900}})).unwrap();
    let before = serde_json::to_value(&*s.doc().unwrap().doc).unwrap();
    for p in [
        json!({"values": {"2bust": 900}}),
        json!({"values": {"bust": -5}}),
        json!({"values": {"bust": "lots"}}),
        json!({"formulas": {"ease": "4 +"}}),
        json!({"formulas": {"ease": 40}}),
        json!({"share": "yes"}),
    ] {
        assert!(s.execute("measure.set", &p).is_err(), "{p}");
    }
    assert!(s.execute("draft.setPoint", &json!({"id": path.0, "anchor": 9, "x": "bust"})).is_err());
    assert!(s.execute("draft.setPoint", &json!({"id": path.0, "anchor": 0, "x": "bust +"})).is_err());
    assert!(s.execute("measure.importChart", &json!({"data": "size\n"})).is_err());
    assert!(s.execute("measure.importChart", &json!({"data": "size,bust\nS,1\n", "unit": "furlong"})).is_err());
    assert_eq!(serde_json::to_value(&*s.doc().unwrap().doc).unwrap(), before);
    // A formula naming a measurement the set doesn't have leaves the point where it is, and says so.
    let d = s.execute("draft.setPoint", &json!({"id": path.0, "anchor": 1, "x": "hip / 4"})).unwrap();
    assert!(d["points"][0]["errors"][0].as_str().unwrap().contains("hip"), "{d}");
    assert!(near(anchor(&s, path, 1).0, 200.0));
}

/// The document an exported SVG or PDF carries for Preserve Editing, opened again.
fn reopened(s: &mut Session, cmd: &str, p: Value) -> Value {
    let r = s.execute(cmd, &p).unwrap();
    let name = if cmd.contains("Pdf") { "shared.pdf" } else { "shared.svg" };
    let mut t = Session::new();
    let o = t.execute("document.open", &json!({"name": name, "dataBase64": r["dataBase64"]})).unwrap();
    assert_eq!(o["restored"], json!(true), "{o}");
    t.execute("measure.info", &json!({})).unwrap()
}

#[test]
fn personal_measurements_stay_out_of_shared_files_unless_shared() {
    let mut s = session();
    bodice(&mut s);
    s.execute("measure.set", &json!({"name": "Me", "values": {"bust": 920}, "formulas": {"ease": "40"}})).unwrap();
    for (cmd, p) in [("document.exportPdf", json!({"preserveEditing": true})), ("document.export", json!({"format": "svg", "preserveEditing": true}))]
    {
        let m = reopened(&mut s, cmd, p);
        assert_eq!(m["values"], json!({}), "{cmd}: the measured values are left out");
        assert_eq!(m["formulas"], json!({"ease": "40"}), "{cmd}: the pattern's formulas stay");
    }
    // The user chooses to share them.
    s.execute("measure.set", &json!({"share": true})).unwrap();
    let m = reopened(&mut s, "document.exportPdf", json!({"preserveEditing": true}));
    assert_eq!(m["values"], json!({"bust": 920.0}));
    // The document itself keeps them.
    assert_eq!(s.execute("measure.info", &json!({})).unwrap()["values"], json!({"bust": 920.0}));
}

/// A document whose measurement set was edited outside the app (a name, a length or a formula
/// `measure.set` refuses) is refused, as `measure.set` would refuse it; the set as saved opens.
#[test]
fn measurements_read_from_a_file_are_checked_like_measure_set() {
    let mut s = session();
    s.execute("measure.set", &json!({"name": "Me", "values": {"bust": 900}, "formulas": {"ease": "40"}})).unwrap();
    let bytes = vectorcraft_format::save(&s.doc().unwrap().doc, false);
    let open = |v: &Value| {
        let data = vectorcraft_format::base64_encode(&serde_json::to_vec(v).unwrap());
        let mut t = Session::new();
        t.execute("document.open", &json!({"name": "edited.patternfoundry", "dataBase64": data})).map(|_| t)
    };
    let saved: Value = serde_json::from_slice(&bytes).unwrap();
    let mut t = open(&saved).unwrap();
    let info = t.execute("measure.info", &json!({})).unwrap();
    assert_eq!((&info["values"], &info["formulas"]), (&json!({"bust": 900.0}), &json!({"ease": "40"})), "{info}");
    for (key, edit) in [
        ("values", json!({"bust": -5})),
        ("values", json!({"bust": 1e300})),
        ("values", json!({"2bust": 900})),
        ("formulas", json!({"ease": "4 +"})),
        ("formulas", json!({"not a name": "1"})),
    ] {
        let mut v = saved.clone();
        find_key(&mut v, "measurements").expect("the document keeps its measurements")[key] = edit.clone();
        assert!(open(&v).is_err(), "{key}: {edit}");
    }
}

/// The first value under `key` anywhere in `v`.
fn find_key<'a>(v: &'a mut Value, key: &str) -> Option<&'a mut Value> {
    match v {
        Value::Object(o) => {
            if o.get(key).is_some_and(|m| !m.is_null()) {
                return o.get_mut(key);
            }
            o.values_mut().find_map(|c| find_key(c, key))
        }
        Value::Array(a) => a.iter_mut().find_map(|c| find_key(c, key)),
        _ => None,
    }
}
