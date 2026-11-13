//! DXF-AAMA export (PF-7.1, `document.export {format: aama}`): pattern pieces as an ASTM
//! D6673 file, at true scale.

use pf_core::mm_to_pt;
use serde_json::{Value, json};

use super::*;

/// A 10 cm square made a piece, with a 10 mm allowance and a notch in the middle of its top edge.
fn square_piece(cut: u32, mirror: bool) -> Session {
    let mut s = Session::new();
    s.execute("file.new", &json!({"width": 800, "height": 800})).unwrap();
    let side = mm_to_pt(100.0);
    s.execute("shape.rectangle", &json!({"x": 40, "y": 40, "width": side, "height": side})).unwrap();
    s.execute("piece.make", &json!({"name": "Front", "allowance": 10})).unwrap();
    s.execute("piece.set", &json!({"cut": cut, "mirror": mirror, "fabric": "Main"})).unwrap();
    s.execute("piece.addNotch", &json!({"edge": 0, "at": 0.5})).unwrap();
    s
}

fn export(s: &mut Session, p: Value) -> String {
    let r = s.execute("document.export", &p).unwrap();
    // DXF lines end in CR LF; the checks below read lines.
    String::from_utf8(vectorcraft_format::base64_decode(r["dataBase64"].as_str().unwrap()).unwrap()).unwrap().replace("\r\n", "\n")
}

/// The `(code, value)` pairs of a DXF file.
fn pairs(s: &str) -> Vec<(i32, String)> {
    let lines: Vec<&str> = s.lines().collect();
    lines.chunks(2).filter_map(|c| Some((c.first()?.trim().parse().ok()?, c.get(1)?.trim().to_string()))).collect()
}

/// Each entity of type `kind` on `layer`: its pairs.
fn entities(s: &str, kind: &str, layer: &str) -> Vec<Vec<(i32, String)>> {
    let p = pairs(s);
    let mut out = Vec::new();
    for (i, x) in p.iter().enumerate() {
        if *x == (0, kind.to_string()) {
            let body: Vec<(i32, String)> = p[i + 1..].iter().take_while(|y| y.0 != 0).cloned().collect();
            if body.iter().any(|y| *y == (8, layer.to_string())) {
                out.push(body);
            }
        }
    }
    out
}

fn values(e: &[(i32, String)], code: i32) -> Vec<f64> {
    e.iter().filter(|p| p.0 == code).filter_map(|p| p.1.parse().ok()).collect()
}

fn span(xs: &[f64]) -> f64 {
    xs.iter().cloned().fold(f64::NEG_INFINITY, f64::max) - xs.iter().cloned().fold(f64::INFINITY, f64::min)
}

#[test]
fn the_10_cm_square_is_100_mm_on_the_sew_line_and_120_on_the_cutting_line() {
    let mut s = square_piece(1, false);
    for version in ["R13", "R12"] {
        let dxf = export(&mut s, json!({"format": "aama", "version": version, "styleName": "Test", "sampleSize": "M"}));
        assert!(dxf.contains(if version == "R13" { "AC1012" } else { "AC1009" }));
        // True scale: the seam line (layer 14) of the 10 cm square measures exactly 100 mm, the
        // cutting line (layer 1) 120 with 10 mm either side.
        let verts = |layer: &str| entities(&dxf, "VERTEX", layer);
        let xs = |layer: &str| verts(layer).iter().flat_map(|e| values(e, 10)).collect::<Vec<_>>();
        let ys = |layer: &str| verts(layer).iter().flat_map(|e| values(e, 20)).collect::<Vec<_>>();
        assert!((span(&xs("14")) - 100.0).abs() < 1e-6 && (span(&ys("14")) - 100.0).abs() < 1e-6, "{:?}", xs("14"));
        assert!((span(&xs("1")) - 120.0).abs() < 1e-6 && (span(&ys("1")) - 120.0).abs() < 1e-6);
        assert_eq!((xs("84"), xs("87")), (xs("1"), xs("14")));
        // The notch, on the cutting line at the middle of the top edge, 6 mm deep, cut straight
        // down (y up: -90°).
        let notch = &entities(&dxf, "POINT", "4")[0];
        let top = ys("1").iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        assert!(
            (values(notch, 20)[0] - top).abs() < 1e-6
                && (values(notch, 10)[0] - (xs("1").iter().cloned().fold(f64::INFINITY, f64::min) + 60.0)).abs() < 1e-6
        );
        assert_eq!((values(notch, 30), values(notch, 50)), (vec![6.0], vec![-90.0]));
        assert_eq!(entities(&dxf, "LINE", "7").len(), 1);
        // The texts.
        for line in ["Piece Name: Front", "Quantity: 1,0", "Material: Main", "Style Name: Test", "Sample Size: M", "Units: METRIC"] {
            assert!(dxf.contains(&format!("\n{line}\n")), "{line}");
        }
    }
}

#[test]
fn mirrored_pairs_are_right_and_left_pieces_and_a_document_without_pieces_is_refused() {
    let mut s = square_piece(2, true);
    assert!(export(&mut s, json!({"format": "aama"})).contains("\nQuantity: 1,1\n"));
    assert!(s.execute("document.export", &json!({"format": "aama", "version": "2018"})).is_err());
    let mut t = Session::new();
    t.execute("file.new", &json!({"width": 800, "height": 800})).unwrap();
    t.execute("shape.rectangle", &json!({"x": 0, "y": 0, "width": 100, "height": 100})).unwrap();
    let e = t.execute("document.export", &json!({"format": "aama"})).unwrap_err().to_string();
    assert!(e.contains("pattern pieces"), "{e}");
}

#[test]
fn unnamed_pieces_go_by_their_block_part_and_every_name_is_once_in_the_file() {
    let mut s = Session::new();
    s.execute("file.new", &json!({"width": 1400, "height": 1000})).unwrap();
    s.execute("measure.set", &json!({"values": {"waist": 680, "hip": 940, "waist_to_hip": 200, "skirt_length": 600}})).unwrap();
    s.execute("block.skirt", &json!({"origin": [0, 0]})).unwrap();
    s.execute("select.all", &json!({})).unwrap();
    s.execute("piece.make", &json!({})).unwrap();
    let side = mm_to_pt(100.0);
    for _ in 0..2 {
        s.execute("shape.rectangle", &json!({"x": 900, "y": 0, "width": side, "height": side})).unwrap();
        s.execute("piece.make", &json!({"name": "Pocket"})).unwrap();
    }
    let dxf = export(&mut s, json!({"format": "aama"}));
    for name in ["Back Skirt Block", "Front Skirt Block", "Pocket", "Pocket 2"] {
        assert!(dxf.contains(&format!("\nPiece Name: {name}\n")), "{name}");
    }
}

#[test]
fn a_piece_cut_on_the_fold_is_the_half_with_its_mirror_line_on_layer_6() {
    let mut s = square_piece(1, false);
    // The left side (edge 3) on the fold.
    s.execute("piece.set", &json!({"fold": 3})).unwrap();
    let dxf = export(&mut s, json!({"format": "aama"}));
    let verts = |layer: &str| entities(&dxf, "VERTEX", layer);
    let xs = |layer: &str| verts(layer).iter().flat_map(|e| values(e, 10)).collect::<Vec<_>>();
    // The half: 110 mm across (no allowance on the fold), the seam line still 100.
    assert!((span(&xs("1")) - 110.0).abs() < 1e-6 && (span(&xs("14")) - 100.0).abs() < 1e-6, "{:?}", xs("1"));
    // The mirror line: down the fold, where the cutting line and the seam line meet, 100 mm long.
    let m = entities(&dxf, "LINE", "6");
    assert_eq!(m.len(), 1);
    let (x, y) = ([values(&m[0], 10)[0], values(&m[0], 11)[0]], [values(&m[0], 20)[0], values(&m[0], 21)[0]]);
    let left = xs("1").iter().cloned().fold(f64::INFINITY, f64::min);
    assert!((x[0] - left).abs() < 1e-6 && (x[1] - left).abs() < 1e-6 && ((y[0] - y[1]).abs() - 100.0).abs() < 1e-6, "{x:?} {y:?}");
    // Without a fold there is none.
    s.execute("piece.set", &json!({"fold": null})).unwrap();
    assert!(entities(&export(&mut s, json!({"format": "aama"})), "LINE", "6").is_empty());
}

#[test]
fn drill_holes_are_points_on_layer_13_with_their_diameter() {
    let mut s = square_piece(1, false);
    // At the middle of the square, a 4 mm punch.
    let c = 40.0 + mm_to_pt(50.0);
    s.execute("piece.addDrill", &json!({"point": [c, c], "diameter": 4})).unwrap();
    let dxf = export(&mut s, json!({"format": "aama"}));
    let d = entities(&dxf, "POINT", "13");
    assert_eq!(d.len(), 1);
    // The cutting line starts at x 0; the middle is 10 + 50 mm in, and 60 mm up from its bottom,
    // which is 60 mm above the style text.
    let (x, y, z) = (values(&d[0], 10)[0], values(&d[0], 20)[0], values(&d[0], 30)[0]);
    assert!((x - 60.0).abs() < 1e-6 && (y - 120.0).abs() < 1e-6 && (z - 4.0).abs() < 1e-6, "{x} {y} {z}");
    // It reads back.
    let back = pf_aama::read(dxf.replace('\n', "\r\n").as_bytes()).unwrap();
    assert_eq!(back.pieces[0].drills.len(), 1);
    // A piece cut on the fold, opened out, has it on both halves.
    s.execute("piece.set", &json!({"fold": 3})).unwrap();
    let out = s.execute("piece.outlines", &json!({})).unwrap();
    assert_eq!(out["pieces"][0]["drills"].as_array().unwrap().len(), 2);
    assert_eq!(out["pieces"][0]["drills"][0]["diameter"], json!(4.0));
}

#[test]
fn t_castle_and_u_notches_are_points_on_their_own_layers() {
    let mut s = square_piece(1, false);
    // The square's one notch, on the top edge, made a castle notch; a T on the right, a U below.
    s.execute("piece.setNotch", &json!({"index": 0, "kind": "castle"})).unwrap();
    s.execute("piece.addNotch", &json!({"edge": 1, "kind": "t"})).unwrap();
    s.execute("piece.addNotch", &json!({"edge": 2, "kind": "u"})).unwrap();
    assert!(s.execute("piece.setNotch", &json!({"index": 0, "kind": "star"})).is_err());
    assert!(s.execute("piece.setNotch", &json!({"index": 9, "kind": "t"})).is_err());
    let dxf = export(&mut s, json!({"format": "aama"}));
    assert!(entities(&dxf, "POINT", "4").is_empty());
    let castle = &entities(&dxf, "POINT", "81")[0];
    assert_eq!((values(castle, 30), values(castle, 39)), (vec![6.0], vec![3.0]));
    // The T notch's point at its depth: 6 mm in from the cutting line's right side (x 120).
    let t = &entities(&dxf, "POINT", "80")[0];
    assert!((values(t, 10)[0] - 114.0).abs() < 1e-6 && (values(t, 50)[0].to_radians().cos() + 1.0).abs() < 1e-9, "{t:?}");
    assert_eq!(entities(&dxf, "POINT", "83").len(), 1);
    // They read back as they were written.
    let back = pf_aama::read(dxf.replace('\n', "\r\n").as_bytes()).unwrap();
    let shapes: Vec<_> = back.pieces[0].notches.iter().map(|n| n.shape).collect();
    assert_eq!(shapes, [pf_aama::NotchShape::Castle, pf_aama::NotchShape::T, pf_aama::NotchShape::U]);
    assert!((back.pieces[0].notches[1].at[0] - 120.0).abs() < 1e-6);
}

#[test]
fn a_dxf_aama_file_opens_as_its_pattern_pieces() {
    let mut s = square_piece(2, true);
    let c = 40.0 + mm_to_pt(50.0);
    s.execute("piece.addDrill", &json!({"point": [c, c], "diameter": 4})).unwrap();
    s.execute("piece.addNotch", &json!({"edge": 1, "kind": "castle"})).unwrap();
    s.execute("piece.set", &json!({"fold": 3, "edges": {"2": 40}, "grainline": {"from": [c, 60], "to": [c, 260]}})).unwrap();
    let r = s.execute("document.export", &json!({"format": "aama", "styleName": "Square"})).unwrap();
    let data = r["dataBase64"].as_str().unwrap().to_string();
    let opened = s.execute("document.open", &json!({"name": "square.dxf", "dataBase64": data})).unwrap();
    assert_eq!(opened["warnings"], json!([]), "{opened}");
    let list = s.execute("piece.list", &json!({})).unwrap();
    assert_eq!(list["pieces"].as_array().unwrap().len(), 1, "{list}");
    let id = list["pieces"][0]["id"].clone();
    let info = s.execute("piece.info", &json!({"id": id})).unwrap();
    assert_eq!(
        (info["name"].as_str(), info["cut"].as_u64(), info["mirror"].as_bool(), info["fabric"].as_str()),
        (Some("Front"), Some(2), Some(true), Some("Main"))
    );
    // True scale: the seam line is the 10 cm square again (400 mm round), the allowance 10 mm.
    assert!((info["seamLength"].as_f64().unwrap() - 400.0).abs() < 0.01 && (info["allowance"].as_f64().unwrap() - 10.0).abs() < 0.01, "{info}");
    // The left side on the fold, the two notches with their shapes, the drill hole and the grainline.
    assert_eq!((info["fold"].as_u64(), &info["edges"]), (Some(3), &json!({"2": 40.0})), "{info}");
    let kinds: Vec<&str> = info["notches"].as_array().unwrap().iter().filter_map(|n| n["kind"].as_str()).collect();
    assert!(kinds.contains(&"slit") && kinds.contains(&"castle"), "{kinds:?}");
    assert_eq!(info["drills"][0]["diameter"], json!(4.0));
    assert!(info["grainline"].is_object());
    // Written out again: 110 mm across (no allowance on the fold), 150 mm down (the 40 mm hem).
    let dxf = export(&mut s, json!({"format": "aama"}));
    let xs: Vec<f64> = entities(&dxf, "VERTEX", "1").iter().flat_map(|e| values(e, 10)).collect();
    let ys: Vec<f64> = entities(&dxf, "VERTEX", "1").iter().flat_map(|e| values(e, 20)).collect();
    assert!((span(&xs) - 110.0).abs() < 1e-3 && (span(&ys) - 150.0).abs() < 1e-3, "{} {}", span(&xs), span(&ys));
    assert_eq!(entities(&dxf, "POINT", "13").len(), 1);
    // A plain DXF still opens as a drawing.
    let plain = s.execute("document.export", &json!({"format": "dxf"})).unwrap();
    s.execute("document.open", &json!({"name": "plain.dxf", "dataBase64": plain["dataBase64"]})).unwrap();
    assert_eq!(s.execute("piece.list", &json!({})).unwrap()["pieces"], json!([]));
}

#[test]
fn the_skirt_blocks_curves_come_back_with_one_allowance_each() {
    let mut s = Session::new();
    s.execute("file.new", &json!({"width": 800, "height": 800})).unwrap();
    s.execute("measure.set", &json!({"values": {"waist": 680, "hip": 940, "waist_to_hip": 200, "skirt_length": 600}})).unwrap();
    s.execute("block.skirt", &json!({"origin": [0, 0]})).unwrap();
    s.execute("select.all", &json!({})).unwrap();
    s.execute("piece.make", &json!({})).unwrap();
    let before = s.execute("piece.list", &json!({})).unwrap()["pieces"].as_array().unwrap().len();
    let r = s.execute("document.export", &json!({"format": "aama"})).unwrap();
    let first = String::from_utf8(vectorcraft_format::base64_decode(r["dataBase64"].as_str().unwrap()).unwrap()).unwrap().replace("\r\n", "\n");
    let widths = |dxf: &str| {
        let xs: Vec<f64> = entities(dxf, "VERTEX", "1").iter().flat_map(|e| values(e, 10)).collect();
        let ys: Vec<f64> = entities(dxf, "VERTEX", "1").iter().flat_map(|e| values(e, 20)).collect();
        (span(&xs), span(&ys))
    };
    let opened = s.execute("document.open", &json!({"name": "skirt.dxf", "dataBase64": r["dataBase64"]})).unwrap();
    assert_eq!(opened["warnings"], json!([]), "{opened}");
    let list = s.execute("piece.list", &json!({})).unwrap();
    let pieces = list["pieces"].as_array().unwrap();
    assert_eq!(pieces.len(), before);
    for p in pieces {
        let info = s.execute("piece.info", &json!({"id": p["id"]})).unwrap();
        // The curves flattened into many short edges, each the piece's allowance.
        assert!(info["edgeCount"].as_u64().unwrap() > 10 && info.get("edges").is_none(), "{info}");
        assert!(info["problem"].is_null(), "{info}");
    }
    // True scale: the cutting lines written out again measure as they did, to a fiftieth of a mm.
    let (w0, h0) = widths(&first);
    let (w1, h1) = widths(&export(&mut s, json!({"format": "aama"})));
    assert!((w0 - w1).abs() < 0.02 && (h0 - h1).abs() < 0.02, "{w0}×{h0} then {w1}×{h1}");
}

#[test]
fn a_quantity_too_large_to_add_up_opens_as_the_most_a_piece_can_be_cut() {
    let mut s = square_piece(2, true);
    let dxf = export(&mut s, json!({"format": "aama"})).replace("\nQuantity: 1,1\n", "\nQuantity: 4294967295,1\n");
    assert!(dxf.contains("Quantity: 4294967295,1"));
    let data = vectorcraft_format::base64_encode(dxf.as_bytes());
    s.execute("document.open", &json!({"name": "huge.dxf", "dataBase64": data})).unwrap();
    let list = s.execute("piece.list", &json!({})).unwrap();
    assert_eq!(list["pieces"][0]["cut"], json!(pf_core::piece::MAX_CUT), "{list}");
    assert_eq!(list["pieces"][0]["mirror"], json!(true), "{list}");
}

/// Issue 169: a piece digitised as tens of thousands of short edges on both its cutting line and
/// its sew line opens in moments, with its allowance (before, each seam edge was measured against
/// every cutting edge: 40 000 points took 11 s, and the time grew with the square of the count).
#[test]
fn a_densely_digitised_dxf_aama_piece_opens_quickly() {
    let n = 80_000;
    let ring = |layer: &str, r: f64| {
        let mut o = format!("0\nLWPOLYLINE\n8\n{layer}\n90\n{n}\n70\n1\n");
        for i in 0..n {
            let a = f64::from(i) / f64::from(n) * std::f64::consts::TAU;
            o += &format!("10\n{:.3}\n20\n{:.3}\n", 500.0 + r * a.cos(), 500.0 + r * a.sin());
        }
        o
    };
    let dxf = format!(
        "0\nSECTION\n2\nBLOCKS\n0\nBLOCK\n2\nRound\n0\nTEXT\n8\n1\n1\nPiece Name: Round\n{}{}0\nENDBLK\n0\nENDSEC\n0\nEOF\n",
        ring("1", 310.0),
        ring("14", 300.0)
    );
    let mut s = Session::new();
    let started = std::time::Instant::now();
    let opened = s.execute("document.open", &json!({"name": "round.dxf", "dataBase64": vectorcraft_format::base64_encode(dxf.as_bytes())})).unwrap();
    let took = started.elapsed();
    assert_eq!(opened["warnings"], json!([]), "{opened}");
    let list = s.execute("piece.list", &json!({})).unwrap();
    let info = s.execute("piece.info", &json!({"id": list["pieces"][0]["id"]})).unwrap();
    assert!((info["allowance"].as_f64().unwrap() - 10.0).abs() < 0.01, "{info}");
    assert!(took.as_secs_f64() < 15.0, "opening took {took:?}");
}
