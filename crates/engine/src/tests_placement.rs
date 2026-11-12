//! Designs placed on a piece (PF-12, `piece.addPlacement`…): an embroidery or screen-print design
//! laid on a pattern piece at its true size.

use pf_core::{Mark, PatternTag, mm_to_pt};
use serde_json::{Value, json};
use vectorcraft_doc::{Node, NodeId, NodeKind};

use super::*;

fn cm10() -> f64 {
    mm_to_pt(100.0)
}

/// A session with a 10 cm square piece at (0, 0).
fn piece() -> (Session, NodeId) {
    let mut s = Session::new();
    s.execute("file.new", &json!({"width": 800, "height": 800})).unwrap();
    s.execute("shape.rectangle", &json!({"x": 0, "y": 0, "width": cm10(), "height": cm10()})).unwrap();
    let r = s.execute("piece.make", &json!({})).unwrap();
    (s, NodeId(r["ids"][0].as_u64().unwrap()))
}

fn part(s: &Session, id: NodeId, mark: Mark) -> Option<Node> {
    let g = s.doc().unwrap().doc.node(id).unwrap().clone();
    g.children()?.iter().find(|c| c.extras().pattern.as_deref().and_then(PatternTag::mark) == Some(mark)).map(|c| (**c).clone())
}

fn near(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-6
}

/// A DST header (512 bytes) for a design with these extents, in tenths of a millimetre.
fn dst(px: u32, nx: u32, py: u32, ny: u32) -> Vec<u8> {
    let mut h = format!("LA:ROSE\r+X:{px:>5}\r-X:{nx:>5}\r+Y:{py:>5}\r-Y:{ny:>5}\r").into_bytes();
    h.resize(512, b' ');
    h
}

/// The start of a PNG that is `w` by `h` pixels and declares no resolution.
fn png(w: u32, h: u32) -> Vec<u8> {
    let mut b = b"\x89PNG\r\n\x1a\n\0\0\0\x0dIHDR".to_vec();
    b.extend(w.to_be_bytes());
    b.extend(h.to_be_bytes());
    b.extend([8, 2, 0, 0, 0, 0, 0, 0, 0]);
    b
}

fn place(s: &mut Session, name: &str, bytes: &[u8], more: Value) -> Result<Value, crate::EngineError> {
    let mut p = json!({"name": name, "dataBase64": vectorcraft_format::base64_encode(bytes)});
    for (k, v) in more.as_object().unwrap() {
        p[k] = v.clone();
    }
    s.execute("piece.addPlacement", &p)
}

/// The x and y extent of the first line of the piece's placement marks (the design's outline), mm.
fn outline_mm(s: &Session, g: NodeId) -> (f64, f64) {
    let marks = part(s, g, Mark::Placements).unwrap();
    let NodeKind::Path { path, .. } = &marks.kind else { panic!("not a path") };
    let xs: Vec<f64> = path.subpaths[0].anchors.iter().map(|a| a.p.x).collect();
    let ys: Vec<f64> = path.subpaths[0].anchors.iter().map(|a| a.p.y).collect();
    let span = |v: &[f64]| v.iter().cloned().fold(f64::NEG_INFINITY, f64::max) - v.iter().cloned().fold(f64::INFINITY, f64::min);
    (pf_core::pt_to_mm(span(&xs)), pf_core::pt_to_mm(span(&ys)))
}

#[test]
fn an_embroidery_file_is_placed_at_the_size_its_header_gives() {
    let (mut s, g) = piece();
    let r = place(&mut s, "rose.dst", &dst(450, 450, 300, 300), json!({})).unwrap();
    let placed = &r["placed"];
    assert_eq!((placed["widthMm"].as_f64(), placed["heightMm"].as_f64()), (Some(90.0), Some(60.0)));
    assert_eq!(placed["size"], "90 x 60 mm");
    assert_eq!(placed["sizeIn"], "3.54 x 2.36 in");
    assert_eq!(placed["note"], "Embroider \"rose\" here, 90 x 60 mm");
    assert_eq!(placed["app"], "StitchFoundry");
    // The outline drawn on the pattern measures 90 x 60 mm, true to scale, centred on the piece.
    let (w, h) = outline_mm(&s, g);
    assert!(near(w, 90.0) && near(h, 60.0), "{w} x {h}");
    let marks = part(&s, g, Mark::Placements).unwrap();
    let NodeKind::Path { path, .. } = &marks.kind else { panic!() };
    // The outline's ring and the crosshair's two lines.
    assert_eq!(path.subpaths.len(), 3);
    let mid = path.subpaths[1].anchors.iter().map(|a| a.p.x).sum::<f64>() / 2.0;
    assert!(near(mid, cm10() / 2.0), "{mid}");
}

#[test]
fn a_design_is_sized_to_a_true_width_turned_and_moved_with_its_piece() {
    let (mut s, g) = piece();
    place(&mut s, "rose.dst", &dst(450, 450, 300, 300), json!({})).unwrap();
    let r = s.execute("piece.setPlacement", &json!({"widthMm": 45})).unwrap();
    assert_eq!(r["placed"]["size"], "45 x 30 mm");
    assert_eq!(r["placements"][0]["scale"], 0.5);
    let (w, h) = outline_mm(&s, g);
    assert!(near(w, 45.0) && near(h, 30.0), "{w} x {h}");
    // Turned a quarter: the 45 mm side now runs down the page.
    s.execute("piece.setPlacement", &json!({"rotation": 90})).unwrap();
    let (w, h) = outline_mm(&s, g);
    assert!(near(w, 30.0) && near(h, 45.0), "{w} x {h}");
    // A height in inches; moving the piece moves the design.
    let r = s.execute("piece.setPlacement", &json!({"heightMm": "1 in", "rotation": 0})).unwrap();
    assert_eq!(r["placed"]["size"], "38.1 x 25.4 mm");
    let before = r["placements"][0]["at"]["x"].as_f64().unwrap();
    s.execute("select.set", &json!({"ids": [g.0]})).unwrap();
    s.execute("object.move", &json!({"dx": 10, "dy": 5})).unwrap();
    let info = s.execute("piece.info", &json!({})).unwrap();
    assert!(near(info["placements"][0]["at"]["x"].as_f64().unwrap(), before + 10.0));
    // Moved by mm, and snapped onto the grainline (the vertical through the middle).
    let r = s.execute("piece.setPlacement", &json!({"moveMm": [10, 0]})).unwrap();
    assert!(near(r["placements"][0]["at"]["x"].as_f64().unwrap(), before + 10.0 + mm_to_pt(10.0)));
    let r = s.execute("piece.setPlacement", &json!({"snap": "grain"})).unwrap();
    assert!(near(r["placements"][0]["at"]["x"].as_f64().unwrap(), cm10() / 2.0 + 10.0));
    assert!(s.execute("piece.setPlacement", &json!({"snap": "fold"})).is_err(), "not cut on the fold");
    // Nonsense is refused and changes nothing.
    assert!(s.execute("piece.setPlacement", &json!({"scale": 0})).is_err());
    assert!(s.execute("piece.setPlacement", &json!({"scale": "big"})).is_err());
    assert!(s.execute("piece.setPlacement", &json!({"index": 3})).is_err());
    assert_eq!(s.execute("piece.info", &json!({})).unwrap()["placements"][0]["scale"], r["placements"][0]["scale"]);
    // Removing the only design takes its marks away.
    s.execute("piece.removePlacement", &json!({})).unwrap();
    assert!(part(&s, g, Mark::Placements).is_none());
    assert!(s.execute("piece.removePlacement", &json!({})).is_err());
}

#[test]
fn the_hoop_is_drawn_round_the_design_and_checked_against_it_and_the_piece() {
    let (mut s, g) = piece();
    // A 100 mm hoop round the 90 x 60 design, on a piece 100 mm across: it fits both.
    let r = place(&mut s, "rose.dst", &dst(450, 450, 300, 300), json!({"hoop": "100x100"})).unwrap();
    assert_eq!(r["placed"]["hoop"], json!([100.0, 100.0]));
    assert_eq!(r["placed"]["fitsHoop"], true);
    assert_eq!(r["placed"]["warnings"], json!([]));
    // Four lines now: the design's outline, its crosshair and the hoop's ring, 100 mm square.
    let marks = part(&s, g, Mark::Placements).unwrap();
    let NodeKind::Path { path, .. } = &marks.kind else { panic!() };
    assert_eq!(path.subpaths.len(), 4);
    let xs: Vec<f64> = path.subpaths[3].anchors.iter().map(|a| a.p.x).collect();
    let span = xs.iter().cloned().fold(f64::NEG_INFINITY, f64::max) - xs.iter().cloned().fold(f64::INFINITY, f64::min);
    assert!(near(span, cm10()), "{span}");
    // Scaled up past the hoop: the design no longer fits, and says so.
    let r = s.execute("piece.setPlacement", &json!({"scale": 1.2})).unwrap();
    assert_eq!(r["placed"]["fitsHoop"], false);
    assert!(r["placed"]["warnings"].as_array().unwrap().iter().any(|w| w.as_str().unwrap().contains("larger than the hoop")));
    // A bigger hoop than the piece can take reaches past its cutting line.
    let r = s.execute("piece.setPlacement", &json!({"scale": 1, "hoop": [200, "8 in"]})).unwrap();
    assert_eq!(r["placed"]["fitsHoop"], true);
    assert_eq!(r["placed"]["warnings"], json!(["the hoop reaches past the piece's cutting line"]));
    // No hoop; a hoop that is not a size is refused.
    let r = s.execute("piece.setPlacement", &json!({"hoop": null})).unwrap();
    assert_eq!(r["placed"]["hoop"], Value::Null);
    assert!(s.execute("piece.setPlacement", &json!({"hoop": "round"})).is_err());
    assert!(s.execute("piece.setPlacement", &json!({"hoop": [100]})).is_err());
    assert!(s.execute("piece.setPlacement", &json!({"hoop": [0, 100]})).is_err());
}

#[test]
fn screen_artwork_is_sized_from_its_pixels_or_its_svg_units() {
    let (mut s, _) = piece();
    // 720 x 360 px at the 72 ppi of a file that declares none: 10 x 5 in.
    let r = place(&mut s, "logo.png", &png(720, 360), json!({})).unwrap();
    assert_eq!(r["placed"]["size"], "254 x 127 mm");
    assert_eq!(r["placed"]["note"], "Screen print \"logo\" here, 254 x 127 mm");
    assert_eq!(r["placed"]["app"], "ScreenFoundry");
    let svg = br#"<?xml version="1.0"?><svg xmlns="http://www.w3.org/2000/svg" stroke-width="3" width="100mm" height="50mm" viewBox="0 0 10 5"></svg>"#;
    let r = place(&mut s, "mark.svg", svg, json!({"scale": 2})).unwrap();
    assert_eq!(r["placed"]["size"], "200 x 100 mm");
    // No width or height: the view box, in CSS pixels (96 to the inch).
    let r = place(&mut s, "v.svg", br#"<svg viewBox="0 0 96 48"></svg>"#, json!({"title": "Vee"})).unwrap();
    assert_eq!(r["placed"]["size"], "25.4 x 12.7 mm");
    assert_eq!(r["placed"]["name"], "Vee");
    assert_eq!(r["placements"].as_array().unwrap().len(), 3);
}

#[test]
fn a_file_that_does_not_say_its_size_needs_one_and_a_kind_must_be_known() {
    let (mut s, _) = piece();
    let pes = b"#PES0001 not a real file";
    assert!(place(&mut s, "bird.pes", pes, json!({})).is_err());
    let r = place(&mut s, "bird.pes", pes, json!({"widthMm": "2 in", "heightMm": 30})).unwrap();
    assert_eq!(r["placed"]["size"], "50.8 x 30 mm");
    // One side keeps a file's proportions where the file knows them.
    let r = place(&mut s, "rose.dst", &dst(450, 450, 300, 300), json!({"heightMm": 30})).unwrap();
    assert_eq!(r["placed"]["size"], "45 x 30 mm");
    assert!(place(&mut s, "notes.txt", b"hello", json!({"widthMm": 10, "heightMm": 10})).is_err());
    assert!(place(&mut s, "notes.txt", b"hello", json!({"widthMm": 10, "heightMm": 10, "kind": "laser"})).is_err());
    assert!(place(&mut s, "rose.dst", &dst(450, 450, 300, 300), json!({"scale": 500})).is_err());
    assert!(place(&mut s, "x.png", b"not a png", json!({"widthMm": 0, "heightMm": 10})).is_err());
}
