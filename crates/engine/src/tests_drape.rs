//! Draping (PF-10.3, `drape.make`): pieces sewn together wrapped round the avatar and let fall
//! onto it as cloth.

use pf_core::mm_to_pt;
use serde_json::json;

use super::*;

/// An A-line skirt of two panels (front and back), each 360 mm at the waist and 800 at the hem,
/// 600 long, sewn at both sides, for a body 680 round the waist and 960 round the hips: 720 round
/// at its waist, 1013 at the hip line 200 mm down (5 cm of ease).
fn skirt() -> Session {
    let mut s = Session::new();
    s.execute("file.new", &json!({"width": 2000, "height": 1000})).unwrap();
    s.execute("measure.set", &json!({"values": {"bust": 880, "waist": 680, "hip": 960, "nape_to_waist": 410, "waist_to_hip": 200}})).unwrap();
    let mut ids = Vec::new();
    for (name, x) in [("Front", 0.0), ("Back", 900.0)] {
        let m = |v: f64| mm_to_pt(v);
        let d = format!("M{} 0 L{} 0 L{} {} L{} {} Z", m(x + 220.0), m(x + 580.0), m(x + 800.0), m(600.0), m(x), m(600.0));
        let path = s.execute("path.create", &json!({"d": d})).unwrap()["id"].clone();
        ids.push(s.execute("piece.make", &json!({"ids": [path], "name": name})).unwrap()["ids"][0].clone());
    }
    // Anchors 0 the waist's left end, 1 its right, 2 the hem's right, 3 its left: each side seam
    // runs down one panel's side and up the other's.
    s.execute("piece.sew", &json!({"a": {"id": ids[0], "from": 1, "to": 2}, "b": {"id": ids[1], "from": 3, "to": 0}})).unwrap();
    s.execute("piece.sew", &json!({"a": {"id": ids[1], "from": 1, "to": 2}, "b": {"id": ids[0], "from": 3, "to": 0}})).unwrap();
    s
}

#[test]
fn a_skirt_drapes_on_the_avatar_resting_at_the_waist() {
    let mut s = skirt();
    let r = s.execute("drape.make", &json!({"precision": 20, "steps": 120})).unwrap();
    assert_eq!(r["pieces"], json!(2), "{r}");
    let (top, hem, waist) = (r["top"].as_f64().unwrap(), r["hem"].as_f64().unwrap(), r["waist"].as_f64().unwrap());
    // 720 round at its waist, it settles about at the body's 680 waist (gripping the hips,
    // PF-10.11, its loose rim may stand a little above it) and hangs its 600 mm from there (it
    // doesn't slide off), a few per cent longer under its own weight; its side seams closed, its
    // edges on the seam lines.
    assert!(top < waist + 15.0 && top > waist - 60.0, "{r}");
    assert!((580.0..680.0).contains(&(top - hem)), "{r}");
    assert!(r["widestSeam"].as_f64().unwrap() < 5.0, "{r}");
    assert!(r["worstStretch"].as_f64().unwrap() < 15.0, "{r}");
    // A skirt that fits is told so in plain words, not as a problem.
    assert!(matches!(r["fit"].as_str(), Some("loose" | "snug")), "{r}");
    assert!(!r["fitMessage"].as_str().unwrap().is_empty(), "{r}");
    // The draped cloth as a mesh.
    let dir = std::env::temp_dir().join(format!("pf-drape-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("skirt.obj");
    let r = s.execute("drape.make", &json!({"steps": 10, "path": path.to_str().unwrap()})).unwrap();
    let obj = std::fs::read_to_string(&path).unwrap();
    assert_eq!(obj.lines().filter(|l| l.starts_with("v ")).count(), r["particles"].as_u64().unwrap() as usize);
    std::fs::remove_dir_all(&dir).ok();
}

/// The skirt block made into pieces with `sew` drapes with no `piece.sew` call (the new user's
/// path: Start a Pattern > Skirt, then Drape on Avatar).
#[test]
fn the_skirt_block_made_into_pieces_is_sewn_and_drapes() {
    let mut s = Session::new();
    s.execute("file.new", &json!({"width": 2000, "height": 1000})).unwrap();
    let m = json!({"values": {"bust": 880, "waist": 680, "hip": 960, "nape_to_waist": 410, "waist_to_hip": 200, "skirt_length": 600}});
    s.execute("measure.set", &m).unwrap();
    let ids = s.execute("block.skirt", &json!({})).unwrap()["ids"].clone();
    // Without `sew` the pieces are not sewn, as before.
    let mut plain = Session::new();
    plain.execute("file.new", &json!({"width": 2000, "height": 1000})).unwrap();
    plain.execute("measure.set", &m).unwrap();
    let p = plain.execute("block.skirt", &json!({})).unwrap()["ids"].clone();
    plain.execute("piece.make", &json!({"ids": p})).unwrap();
    assert!(plain.execute("piece.seams", &json!({})).unwrap()["seams"].as_array().unwrap().is_empty());
    assert!(plain.execute("drape.make", &json!({})).unwrap_err().to_string().contains("sewn"));

    s.execute("piece.make", &json!({"ids": ids, "sew": true})).unwrap();
    let seams = s.execute("piece.seams", &json!({})).unwrap();
    let seams = seams["seams"].as_array().unwrap();
    assert_eq!(seams.len(), 1, "{seams:?}");
    assert!(seams[0]["difference"].as_f64().unwrap().abs() < 1.0, "the side seams are as long: {seams:?}");
    let r = s.execute("drape.make", &json!({"precision": 25, "steps": 30, "draw": false})).unwrap();
    assert_eq!(r["pieces"], json!(2), "{r}");
}

#[test]
fn draping_needs_sewn_pieces_and_the_measurements() {
    let mut s = Session::new();
    s.execute("file.new", &json!({"width": 800, "height": 800})).unwrap();
    assert!(s.execute("drape.make", &json!({})).unwrap_err().to_string().contains("bust"));
    s.execute("measure.set", &json!({"values": {"bust": 880, "waist": 680, "hip": 960, "nape_to_waist": 410, "waist_to_hip": 200}})).unwrap();
    assert!(s.execute("drape.make", &json!({})).unwrap_err().to_string().contains("sewn"));
    let mut s = skirt();
    assert!(s.execute("drape.make", &json!({"precision": 1})).is_err());
}

/// The drape view (PF-10.4): the avatar and the draped skirt as two pictures at true scale on the
/// layer "Drape", the front's and the side's, replaced when draped again and gone on undo.
#[test]
fn the_drape_is_drawn_front_and_side_at_true_scale() {
    let mut s = skirt();
    let r = s.execute("drape.make", &json!({"precision": 25, "steps": 30, "origin": [0, 0]})).unwrap();
    let doc = &s.doc().unwrap().doc;
    let image = |id: &serde_json::Value| match &doc.node(NodeId(id.as_u64().unwrap())).unwrap().kind {
        vectorcraft_doc::NodeKind::Image(im) => (**im).clone(),
        _ => panic!("an image"),
    };
    let (front, side) = (image(&r["front"]), image(&r["side"]));
    // 2 mm a pixel, at true scale on the page.
    for im in [&front, &side] {
        assert!((im.xf.as_coeffs()[0] - mm_to_pt(2.0)).abs() < 1e-9 && (im.xf.as_coeffs()[3] - mm_to_pt(2.0)).abs() < 1e-9);
        assert!(doc.images.contains_key(&im.key));
    }
    // From the front as broad as the skirt's hem or the body, whichever is broader (some
    // hundreds of mm); from the side as tall: the neck to the ankle.
    let fw = front.width as f64 * 2.0;
    assert!((300.0..1200.0).contains(&fw), "{fw}");
    assert_eq!(front.height, side.height);
    assert!(side.xf.as_coeffs()[4] > front.xf.as_coeffs()[4] + mm_to_pt(fw), "side by side");
    assert!(r["tightest"].as_f64().is_some(), "{r}");
    // Draped again it replaces the last; undone, the earlier one is back; not drawn when asked.
    s.execute("drape.make", &json!({"precision": 25, "steps": 5})).unwrap();
    let layers = |s: &Session| s.doc().unwrap().doc.layers.iter().filter(|l| l.name.as_deref() == Some("Drape")).count();
    assert_eq!(layers(&s), 1);
    s.execute("edit.undo", &json!({})).unwrap();
    s.execute("edit.undo", &json!({})).unwrap();
    assert_eq!(layers(&s), 0);
    let r = s.execute("drape.make", &json!({"precision": 25, "steps": 5, "draw": false})).unwrap();
    assert!(r.get("layer").is_none() && layers(&s) == 0);
}

/// glTF export (PF-10.5): the avatar and the draped skirt in one binary glTF file, the cloth in
/// the drape view's colours, in metres with y up.
#[test]
fn the_drape_exports_as_gltf_with_the_avatar() {
    let mut s = skirt();
    let dir = std::env::temp_dir().join(format!("pf-drape-glb-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("skirt.glb");
    let r = s.execute("drape.make", &json!({"precision": 25, "steps": 30, "draw": false, "path": path.to_str().unwrap()})).unwrap();
    let bytes = std::fs::read(&path).unwrap();
    let (gltf, bin) = pf_core::gltf::read_glb(&bytes).unwrap();
    assert_eq!(gltf["nodes"][0]["name"], "Avatar");
    assert_eq!(gltf["nodes"][1]["name"], "Drape");
    let cloth = &gltf["meshes"][1]["primitives"][0]["attributes"];
    let pos = &gltf["accessors"][cloth["POSITION"].as_u64().unwrap() as usize];
    assert_eq!(pos["count"].as_u64(), r["particles"].as_u64());
    assert!(cloth.get("COLOR_0").is_some());
    // The cloth's height, in metres up y, as drape.make reports it in mm.
    let top = pos["max"][1].as_f64().unwrap() * 1000.0;
    assert!((top - r["top"].as_f64().unwrap()).abs() < 0.2, "{top} {r}");
    assert_eq!(gltf["buffers"][0]["byteLength"].as_u64().unwrap() as usize, bin.len());
    // Without a path it comes back as data; a format it can't write is refused.
    let r = s.execute("drape.make", &json!({"precision": 30, "steps": 5, "draw": false, "format": "glb"})).unwrap();
    let data = vectorcraft_format::base64_decode(r["dataBase64"].as_str().unwrap()).unwrap();
    assert!(pf_core::gltf::read_glb(&data).is_some());
    let r = s.execute("drape.make", &json!({"precision": 30, "steps": 5, "draw": false, "format": "OBJ"})).unwrap();
    assert!(r["data"].as_str().unwrap().contains("\nf "));
    assert!(s.execute("drape.make", &json!({"precision": 30, "steps": 5, "draw": false, "format": "fbx"})).is_err());
    std::fs::remove_dir_all(&dir).ok();
}

/// The bodice block's back and front, each cut on the fold, sewn at the sides and shoulders, and
/// the sleeve block cut as a mirror-image pair (the piece ids: back 5, front 9, sleeve 13).
fn bodice(sleeves: bool) -> Session {
    let mut s = Session::new();
    s.execute("file.new", &json!({"width": 3370, "height": 2384})).unwrap();
    let m = json!({"values": {"bust": 880, "waist": 680, "hip": 940, "nape_to_waist": 410, "back_width": 344, "chest": 324, "shoulder": 121, "neck": 370,
        "armscye_depth": 210, "waist_to_hip": 206, "top_arm": 280, "sleeve_length": 590}});
    s.execute("measure.set", &m).unwrap();
    s.execute("block.bodice", &json!({})).unwrap();
    s.execute("block.sleeve", &json!({"origin": [1600, 100]})).unwrap();
    for (id, name) in [(2, "Back Bodice"), (3, "Front Bodice"), (4, "Sleeve")] {
        s.execute("piece.make", &json!({"ids": [id], "name": name})).unwrap();
    }
    s.execute("piece.set", &json!({"id": 5, "fold": 0})).unwrap();
    s.execute("piece.set", &json!({"id": 9, "fold": 0})).unwrap();
    s.execute("piece.set", &json!({"id": 13, "cut": 2, "mirror": true})).unwrap();
    let sew = |s: &mut Session, a: (u64, usize, usize), b: (u64, usize, usize)| {
        s.execute("piece.sew", &json!({"a": {"id": a.0, "from": a.1, "to": a.2}, "b": {"id": b.0, "from": b.1, "to": b.2}})).unwrap();
    };
    // The side seams, the shoulders (their darts between), …
    sew(&mut s, (5, 9, 10), (9, 5, 6));
    sew(&mut s, (5, 2, 6), (9, 9, 12));
    if sleeves {
        // …the sleeve's underarm seam and its crown into the front and back armholes.
        sew(&mut s, (13, 4, 5), (13, 9, 10));
        sew(&mut s, (13, 0, 4), (9, 6, 9));
        sew(&mut s, (13, 10, 0), (5, 6, 9));
    }
    s
}

/// A bodice (PF-10.6) hangs from the shoulders to about the waist, opened out across its folds,
/// its four darts (two in each piece) sewn shut and its seams closed.
#[test]
fn a_bodice_drapes_from_the_shoulders() {
    let mut s = bodice(false);
    let r = s.execute("drape.make", &json!({"precision": 25, "steps": 150, "draw": false})).unwrap();
    assert_eq!(r["pieces"], json!(2), "{r}");
    // Each piece opened out: both sides' darts.
    assert_eq!(r["darts"], json!(8), "{r}");
    let (top, hem, waist) = (r["top"].as_f64().unwrap(), r["hem"].as_f64().unwrap(), r["waist"].as_f64().unwrap());
    let a = pf_core::avatar::avatar(&pf_core::avatar::Body {
        bust: 880.0,
        waist: 680.0,
        hip: 940.0,
        nape_to_waist: 410.0,
        waist_to_hip: 206.0,
        neck: Some(370.0),
        shoulder: Some(121.0),
        top_arm: Some(280.0),
        sleeve_length: Some(590.0),
        armscye_depth: Some(210.0),
        ..Default::default()
    })
    .unwrap();
    // At the base of the neck at the top, the waist at the bottom (a little above: the avatar has
    // no bust for the bust dart to shape).
    assert!((top - a.neck_height).abs() < 30.0, "{r}");
    assert!(hem > waist - 30.0 && hem < waist + 50.0, "{r}");
    assert!(r["widestSeam"].as_f64().unwrap() < 12.0, "{r}");
    assert!(r["worstStretch"].as_f64().unwrap() < 40.0, "{r}");
}

/// Sleeves (PF-10.6) go round the arms of the armholes they are sewn into and hang down them; the
/// bodice stays on the shoulders.
#[test]
fn sleeves_drape_round_the_arms() {
    let mut s = bodice(true);
    let dir = std::env::temp_dir().join(format!("pf-drape-sleeves-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("top.obj");
    let r = s.execute("drape.make", &json!({"precision": 25, "steps": 150, "draw": false, "path": path.to_str().unwrap()})).unwrap();
    assert_eq!(r["pieces"], json!(3), "{r}");
    let (top, hem, waist) = (r["top"].as_f64().unwrap(), r["hem"].as_f64().unwrap(), r["waist"].as_f64().unwrap());
    // Still on the shoulders; the sleeves down past the waist.
    assert!(top > waist + 350.0, "{r}");
    assert!(hem < waist - 150.0, "{r}");
    // Set in after the bodice is on (PF-10.9), round arms that grow from the shoulder joint: the
    // armpit no longer pinched 5 to 6 times its size.
    assert!(r["tightest"].as_f64().unwrap() < 400.0, "{r}");
    // Cloth out on both arms, as far out as the sleeves hang.
    let obj = std::fs::read_to_string(&path).unwrap();
    let xs: Vec<f64> = obj.lines().filter_map(|l| l.strip_prefix("v ")).filter_map(|l| l.split(' ').next()?.parse::<f64>().ok()).collect();
    let (lo, hi) = xs.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), x| (a.min(*x), b.max(*x)));
    assert!(lo < -0.3 && hi > 0.3, "{lo} {hi}");
    std::fs::remove_dir_all(&dir).ok();
}

/// The trouser block (front and back, each cut as a mirror-image pair) sewn at the side seams and
/// inseams, and its centre front and centre back each to its mirror (PF-10.10).
fn trousers() -> Session {
    trousers_for(json!({"values": {"bust": 880, "waist": 700, "hip": 960, "nape_to_waist": 410, "waist_to_hip": 200, "crotch_depth": 260,
        "waist_to_knee": 600, "waist_to_ankle": 1040, "trouser_hem": 460}}))
}

/// [`trousers`] drafted from measurement set `m` (`measure.set`'s params).
fn trousers_for(m: serde_json::Value) -> Session {
    let mut s = Session::new();
    s.execute("file.new", &json!({"width": 3370, "height": 2384})).unwrap();
    s.execute("measure.set", &m).unwrap();
    s.execute("block.trousers", &json!({})).unwrap();
    let mut ids = Vec::new();
    for (id, name) in [(2, "Front Trousers"), (3, "Back Trousers")] {
        let r = s.execute("piece.make", &json!({"ids": [id], "name": name})).unwrap();
        let piece = r["ids"][0].as_u64().unwrap();
        s.execute("piece.set", &json!({"id": piece, "cut": 2, "mirror": true})).unwrap();
        ids.push(piece);
    }
    let (f, b) = (ids[0], ids[1]);
    let sew = |s: &mut Session, a: (u64, usize, usize), b: (u64, usize, usize)| {
        s.execute("piece.sew", &json!({"a": {"id": a.0, "from": a.1, "to": a.2}, "b": {"id": b.0, "from": b.1, "to": b.2}})).unwrap();
    };
    // The front: 0 the centre front at the waist, 2 the fork, 4 and 5 the hem, 9 the side at the
    // waist. The back: 0 the centre back at the waist, 4 the side at the waist, 8 and 9 the hem,
    // 11 the fork.
    sew(&mut s, (f, 5, 9), (b, 4, 8));
    sew(&mut s, (f, 2, 4), (b, 9, 11));
    sew(&mut s, (f, 0, 2), (f, 0, 2));
    sew(&mut s, (b, 11, 0), (b, 11, 0));
    s
}

/// Trousers (PF-10.10) hang from the waist down both legs, their back darts sewn, the seams
/// closed: a tube round each leg, the crotch between.
#[test]
fn trousers_drape_down_both_legs() {
    let mut s = trousers();
    let dir = std::env::temp_dir().join(format!("pf-drape-trousers-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("trousers.obj");
    let r = s.execute("drape.make", &json!({"precision": 25, "steps": 150, "draw": false, "path": path.to_str().unwrap()})).unwrap();
    assert_eq!(r["pieces"], json!(2), "{r}");
    assert_eq!(r["darts"], json!(2), "{r}");
    let (top, hem, waist) = (r["top"].as_f64().unwrap(), r["hem"].as_f64().unwrap(), r["waist"].as_f64().unwrap());
    // At the waist (PF-10.19: before, the thighs overlapped across the middle and pushed the
    // crotch seam, and the trousers with it, 65 mm down).
    assert!(top > waist - 30.0 && top < waist + 30.0, "{r}");
    assert!(hem < waist - 900.0, "{r}");
    assert!(r["widestSeam"].as_f64().unwrap() < 15.0, "{r}");
    assert!(r["worstStretch"].as_f64().unwrap() < 40.0, "{r}");
    // Two legs: at the knee, cloth on both sides and none between them.
    let obj = std::fs::read_to_string(&path).unwrap();
    let knee_z = (waist - 600.0) / 1000.0;
    let near_knee: Vec<f64> = obj
        .lines()
        .filter_map(|l| l.strip_prefix("v "))
        .filter_map(|l| {
            let v: Vec<f64> = l.split(' ').filter_map(|x| x.parse().ok()).collect();
            (v.len() == 3 && (v[1] - knee_z).abs() < 0.03).then_some(v[0])
        })
        .collect();
    assert!(near_knee.iter().any(|x| *x > 0.05) && near_knee.iter().any(|x| *x < -0.05), "{near_knee:?}");
    // (Loose round the knee, which is 23 mm from the middle: within a few mm of it, no more.)
    assert!(near_knee.iter().all(|x| x.abs() > 0.003), "{near_knee:?}");
    std::fs::remove_dir_all(&dir).ok();
}

/// A run sewn to itself is refused unless its piece is cut as a mirror-image pair.
#[test]
fn a_centre_seam_needs_a_mirror_image_pair() {
    let mut t = Session::new();
    t.execute("file.new", &json!({"width": 800, "height": 800})).unwrap();
    let path = t.execute("path.create", &json!({"d": "M0 0 L100 0 L100 100 L0 100 Z"})).unwrap()["id"].clone();
    let id = t.execute("piece.make", &json!({"ids": [path]})).unwrap()["ids"][0].clone();
    let same = json!({"a": {"id": id, "from": 0, "to": 1}, "b": {"id": id, "from": 0, "to": 1}});
    assert!(t.execute("piece.sew", &same).unwrap_err().to_string().contains("same run"));
    t.execute("piece.set", &json!({"id": id, "cut": 2, "mirror": true})).unwrap();
    assert!(t.execute("piece.sew", &same).is_ok());
}

/// Cloth grips the body (PF-10.11, static friction): trousers for a man, whose waist is only 21%
/// less round than his hips, stay on him (with the cloth only slowed where it touched, they crept
/// down off the hips to the floor).
#[test]
fn trousers_stay_on_a_male_figure() {
    let mut s = trousers_for(json!({"figure": "male", "values": {"bust": 1000, "waist": 790, "hip": 1000, "nape_to_waist": 470, "waist_to_hip": 190,
        "crotch_depth": 270, "waist_to_knee": 620, "waist_to_ankle": 1080, "trouser_hem": 480}}));
    let r = s.execute("drape.make", &json!({"precision": 25, "steps": 150, "draw": false})).unwrap();
    let (top, waist) = (r["top"].as_f64().unwrap(), r["waist"].as_f64().unwrap());
    // At his waist (PF-10.19; before, about 150 mm below it, on his hips).
    assert!(top > waist - 40.0, "{r}");
    // (Their hems fold over the feet, PF-10.14, where the seams open a little more.)
    assert!(r["widestSeam"].as_f64().unwrap() < 20.0, "{r}");
}

/// The men's trouser block (PF-4.12) drafts for a man and drapes on the male figure down both
/// legs, its seams closed, its back darts sewn, its outlines' runs as long as each other's.
#[test]
fn the_mens_trouser_block_drapes_on_a_male_figure() {
    let mut s = Session::new();
    s.execute("file.new", &json!({"width": 3370, "height": 2384})).unwrap();
    let m = json!({"figure": "male", "values": {"bust": 1000, "waist": 860, "hip": 1020, "nape_to_waist": 470, "waist_to_hip": 190,
        "crotch_depth": 280, "waist_to_knee": 620, "waist_to_ankle": 1080, "trouser_hem": 500}});
    s.execute("measure.set", &m).unwrap();
    assert_eq!(s.execute("block.measurements", &json!({"block": "mensTrousers"})).unwrap()["missing"], json!([]));
    let r = s.execute("block.mensTrousers", &json!({})).unwrap();
    let ids: Vec<u64> = r["ids"].as_array().unwrap().iter().map(|v| v.as_u64().unwrap()).collect();
    let mut pieces = Vec::new();
    for (id, name) in [(ids[0], "Front Trousers"), (ids[1], "Back Trousers")] {
        let piece = s.execute("piece.make", &json!({"ids": [id], "name": name})).unwrap()["ids"][0].as_u64().unwrap();
        s.execute("piece.set", &json!({"id": piece, "cut": 2, "mirror": true})).unwrap();
        pieces.push(piece);
    }
    let (f, b) = (pieces[0], pieces[1]);
    // The front: 0 the centre front's top, 2 the fork, 4 and 5 the hem, 8 the side's top. The
    // back: 0 the centre back's top, 4 the side's top, 7 and 8 the hem, 10 the fork.
    let mut lengths = Vec::new();
    for (a, c) in [((f, 5, 8), (b, 4, 7)), ((f, 2, 4), (b, 8, 10)), ((f, 0, 2), (f, 0, 2)), ((b, 10, 0), (b, 10, 0))] {
        let r = s.execute("piece.sew", &json!({"a": {"id": a.0, "from": a.1, "to": a.2}, "b": {"id": c.0, "from": c.1, "to": c.2}})).unwrap();
        lengths.push(r["difference"].as_f64().unwrap());
    }
    // The side seams and the inside legs as long front and back (within 3 mm).
    assert!(lengths.iter().all(|d| d.abs() < 3.0), "{lengths:?}");
    let r = s.execute("drape.make", &json!({"precision": 25, "steps": 150, "draw": false})).unwrap();
    assert_eq!((r["pieces"].clone(), r["darts"].clone()), (json!(2), json!(2)), "{r}");
    let (top, waist) = (r["top"].as_f64().unwrap(), r["waist"].as_f64().unwrap());
    // A little below the waist (the block's top is 95 cm round for an 86 cm waist, below a 4 cm
    // waistband): about 40 mm.
    assert!(top > waist - 80.0 && top < waist + 30.0, "{r}");
    assert!(r["widestSeam"].as_f64().unwrap() < 15.0 && r["worstStretch"].as_f64().unwrap() < 40.0, "{r}");
}

/// The men's shirt block (PF-4.13) drafts for a man and hangs from his shoulders on the male
/// figure, longer than the waist (a top longer than the neck to the waist hangs from the neck).
#[test]
fn the_mens_shirt_block_hangs_from_the_shoulders() {
    let mut s = Session::new();
    s.execute("file.new", &json!({"width": 3370, "height": 2384})).unwrap();
    let m = json!({"figure": "male", "values": {"bust": 1000, "waist": 860, "hip": 1020, "nape_to_waist": 450, "waist_to_hip": 190, "neck": 400,
        "back_width": 400, "armscye_depth": 240, "shoulder": 150}});
    s.execute("measure.set", &m).unwrap();
    assert_eq!(s.execute("block.measurements", &json!({"block": "mensShirt"})).unwrap()["missing"], json!([]));
    let r = s.execute("block.mensShirt", &json!({})).unwrap();
    let ids: Vec<u64> = r["ids"].as_array().unwrap().iter().map(|v| v.as_u64().unwrap()).collect();
    let mut pieces = Vec::new();
    // The back on the fold of its centre back (edge 6, the hem's corner to the neck), the front of
    // its centre front (edge 1, the neck to the hem).
    for (id, name, fold) in [(ids[0], "Back Shirt", 6), (ids[1], "Front Shirt", 1)] {
        let piece = s.execute("piece.make", &json!({"ids": [id], "name": name})).unwrap()["ids"][0].as_u64().unwrap();
        s.execute("piece.set", &json!({"id": piece, "fold": fold})).unwrap();
        pieces.push(piece);
    }
    let (b, f) = (pieces[0], pieces[1]);
    // The side seams (back 4–5, front 3–4) and the shoulders (back 1–2, front 6–0).
    for (a, c) in [((b, 4, 5), (f, 3, 4)), ((b, 1, 2), (f, 6, 0))] {
        s.execute("piece.sew", &json!({"a": {"id": a.0, "from": a.1, "to": a.2}, "b": {"id": c.0, "from": c.1, "to": c.2}})).unwrap();
    }
    let r = s.execute("drape.make", &json!({"precision": 25, "steps": 150, "draw": false})).unwrap();
    assert_eq!(r["pieces"], json!(2), "{r}");
    let (top, hem, waist) = (r["top"].as_f64().unwrap(), r["hem"].as_f64().unwrap(), r["waist"].as_f64().unwrap());
    // Up at the base of the neck, the hem about 33 cm below the waist (78 cm long from the nape).
    assert!(top > waist + 400.0, "{r}");
    assert!(hem < waist - 250.0 && hem > waist - 400.0, "{r}");
    assert!(r["widestSeam"].as_f64().unwrap() < 10.0 && r["worstStretch"].as_f64().unwrap() < 25.0, "{r}");
}

/// The men's shirt sleeve (PF-4.14), drafted on the shirt block's armhole and set into it: a
/// shirt with sleeves on the male figure, the sleeves down both arms, the armpit not pinched.
#[test]
fn a_mens_shirt_with_sleeves_drapes_on_a_male_figure() {
    let mut s = Session::new();
    s.execute("file.new", &json!({"width": 3370, "height": 2384})).unwrap();
    let m = json!({"figure": "male", "values": {"bust": 1000, "waist": 860, "hip": 1020, "nape_to_waist": 450, "waist_to_hip": 190, "neck": 400,
        "back_width": 400, "armscye_depth": 240, "shoulder": 150, "sleeve_length": 640, "top_arm": 320}});
    s.execute("measure.set", &m).unwrap();
    s.execute("block.mensShirt", &json!({})).unwrap();
    s.execute("block.mensShirtSleeve", &json!({"origin": [1700, 100]})).unwrap();
    let mut pieces = Vec::new();
    for (id, name) in [(2, "Back Shirt"), (3, "Front Shirt"), (4, "Sleeve")] {
        pieces.push(s.execute("piece.make", &json!({"ids": [id], "name": name})).unwrap()["ids"][0].as_u64().unwrap());
    }
    let (b, f, sl) = (pieces[0], pieces[1], pieces[2]);
    s.execute("piece.set", &json!({"id": b, "fold": 6})).unwrap();
    s.execute("piece.set", &json!({"id": f, "fold": 1})).unwrap();
    s.execute("piece.set", &json!({"id": sl, "cut": 2, "mirror": true})).unwrap();
    // The side seams and shoulders; the sleeve's underarm (2–3 to 6–7), its crown's front half
    // (0–2) into the front armhole (4–6) and its back half (7–0) into the back's (2–4).
    let mut ease = Vec::new();
    for (a, c) in [((b, 4, 5), (f, 3, 4)), ((b, 1, 2), (f, 6, 0)), ((sl, 2, 3), (sl, 6, 7)), ((sl, 0, 2), (f, 4, 6)), ((sl, 7, 0), (b, 2, 4))] {
        let r = s.execute("piece.sew", &json!({"a": {"id": a.0, "from": a.1, "to": a.2}, "b": {"id": c.0, "from": c.1, "to": c.2}})).unwrap();
        ease.push(r["difference"].as_f64().unwrap());
    }
    // Each crown half a little longer than its armhole (the ease to set it in).
    assert!(ease[3] > 5.0 && ease[3] < 40.0 && ease[4] > 5.0 && ease[4] < 40.0, "{ease:?}");
    let dir = std::env::temp_dir().join(format!("pf-drape-shirt-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("shirt.obj");
    // At the app's own precision and length (at 25 mm and 150 steps the coarser cloth can't follow
    // the armpit's curve: it reads 372% there, the women's sleeve 338%).
    let r = s.execute("drape.make", &json!({"draw": false, "path": path.to_str().unwrap()})).unwrap();
    assert_eq!(r["pieces"], json!(3), "{r}");
    let (top, waist) = (r["top"].as_f64().unwrap(), r["waist"].as_f64().unwrap());
    assert!(top > waist + 400.0, "{r}");
    assert!(r["widestSeam"].as_f64().unwrap() < 15.0, "{r}");
    // The armpit not pinched as a sleeve set into the women's bodice is (a crown fitted to its
    // armhole).
    assert!(r["tightest"].as_f64().unwrap() < 150.0, "{r}");
    let obj = std::fs::read_to_string(&path).unwrap();
    let xs: Vec<f64> = obj.lines().filter_map(|l| l.strip_prefix("v ")).filter_map(|l| l.split(' ').next()?.parse::<f64>().ok()).collect();
    let (lo, hi) = xs.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), x| (a.min(*x), b.max(*x)));
    assert!(lo < -0.3 && hi > 0.3, "{lo} {hi}");
    std::fs::remove_dir_all(&dir).ok();
}

/// A PNG of upright stripes, red then blue, each `stripe` pixels wide, `n` of each, 30 high.
fn stripes_png(stripe: u32, n: u32) -> Vec<u8> {
    let img = image::RgbImage::from_fn(stripe * 2 * n, 30, |x, _| {
        if (x / stripe).is_multiple_of(2) { image::Rgb([220, 20, 20]) } else { image::Rgb([20, 20, 220]) }
    });
    let mut buf = Vec::new();
    img.write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Png).unwrap();
    buf
}

/// Fabric from a photo (PF-10.16): an image marked as a fabric photo, its width on the fabric
/// given (else as placed), and the draped skirt drawn again in it at true size: a photo of 10 mm
/// stripes shows 10 mm stripes down the middle of the skirt's front.
#[test]
fn the_drape_is_drawn_in_a_fabric_photo() {
    let mut s = skirt();
    let dir = std::env::temp_dir().join(format!("pf-fabric-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("stripes.png");
    // 4 pixels a stripe, so 10 mm when the photo's 80 pixels are 200 mm of the fabric.
    std::fs::write(&path, stripes_png(4, 10)).unwrap();
    let photo = s.execute("file.place", &json!({"path": path, "link": false, "at": [0, 2000]})).unwrap()["ids"][0].clone();
    std::fs::remove_dir_all(&dir).ok();
    // Not a fabric photo yet: no view in it.
    let r = s.execute("drape.make", &json!({"precision": 25, "steps": 5, "origin": [0, 0]})).unwrap();
    assert!(r.get("fabric").is_none(), "{r}");
    // Its width as placed by default; then as given.
    let placed = pf_core::pt_to_mm(s.doc().unwrap().doc.bounds_of(&[NodeId(photo.as_u64().unwrap())], false).unwrap().width());
    let f = s.execute("fabric.set", &json!({"id": photo})).unwrap();
    assert!((f["width"].as_f64().unwrap() - placed).abs() < 1e-6, "{f}");
    let f = s.execute("fabric.set", &json!({"id": photo, "width": "20 cm"})).unwrap();
    assert_eq!((f["width"].as_f64().unwrap().round(), f["name"].as_str(), f["turns"].as_u64()), (200.0, Some(""), Some(0)));
    assert_eq!(s.execute("fabric.list", &json!({})).unwrap()["fabrics"].as_array().unwrap().len(), 1);
    for bad in [json!({"id": photo, "width": 5}), json!({"id": photo, "turns": 4})] {
        assert!(s.execute("fabric.set", &bad).is_err(), "{bad}");
    }
    let piece = s.execute("piece.list", &json!({})).unwrap()["pieces"][0]["id"].clone();
    assert!(s.execute("fabric.set", &json!({"id": piece, "width": 200})).unwrap_err().to_string().contains("not an image"));
    // Draped, the front again in the fabric, 1 mm a pixel, right of the side view.
    let r = s.execute("drape.make", &json!({"precision": 20, "steps": 60, "origin": [0, 0]})).unwrap();
    let doc = &s.doc().unwrap().doc;
    let image = |id: &serde_json::Value| match &doc.node(NodeId(id.as_u64().unwrap())).unwrap().kind {
        vectorcraft_doc::NodeKind::Image(im) => (**im).clone(),
        _ => panic!("an image"),
    };
    let (side, fabric) = (image(&r["side"]), image(&r["fabric"]));
    assert!((fabric.xf.as_coeffs()[0] - mm_to_pt(1.0)).abs() < 1e-9);
    assert!(fabric.xf.as_coeffs()[4] > side.xf.as_coeffs()[4] + mm_to_pt(side.width as f64 * 2.0), "beside the side view");
    let pic = image::load_from_memory(&doc.images[&fabric.key].bytes).unwrap().to_rgba8();
    // Across the middle of the skirt's front, 150 mm below its top: red and blue stripes, the
    // middle ones (facing the viewer, not foreshortened) 10 mm wide.
    if let Ok(out) = std::env::var("PF_FABRIC_VIEW") {
        pic.save(out).unwrap();
    }
    // Red 1, blue -1, else 0.
    let stripe = |x: u32, y: u32| {
        let p = pic.get_pixel(x, y).0.map(u32::from);
        if p[3] == 0 {
            0
        } else if p[0] > 2 * p[2] + 20 {
            1
        } else if p[2] > 2 * p[0] + 20 {
            -1
        } else {
            0
        }
    };
    let top = (0..pic.height()).find(|y| (0..pic.width()).any(|x| stripe(x, *y) != 0)).unwrap();
    let row: Vec<i8> = (0..pic.width()).map(|x| stripe(x, top + 150)).collect();
    let mut runs: Vec<(u32, i8, u32)> = Vec::new();
    for (x, c) in row.iter().enumerate() {
        match runs.last_mut() {
            Some(r) if r.1 == *c => r.2 += 1,
            _ => runs.push((x as u32, *c, 1)),
        }
    }
    let stripes: Vec<&(u32, i8, u32)> = runs.iter().filter(|r| r.1 != 0).collect();
    assert!(stripes.len() >= 10, "{runs:?}");
    let mid = pic.width() / 2;
    let middle: Vec<u32> = stripes.iter().filter(|r| r.0.abs_diff(mid) < 60).map(|r| r.2).collect();
    assert!(middle.len() >= 4 && middle.iter().all(|w| (8..=12).contains(w)), "{middle:?} of {runs:?}");
    // Cleared, there is no view in it; undone, it is a fabric photo again.
    s.execute("fabric.clear", &json!({"id": photo})).unwrap();
    assert!(s.execute("fabric.clear", &json!({"id": photo})).is_err());
    assert!(s.execute("drape.make", &json!({"precision": 25, "steps": 5})).unwrap().get("fabric").is_none());
    s.execute("edit.undo", &json!({})).unwrap();
    s.execute("edit.undo", &json!({})).unwrap();
    assert_eq!(s.execute("fabric.list", &json!({})).unwrap()["fabrics"].as_array().unwrap().len(), 1);
}

/// A fabric photo's light evened and its repeat made seamless (PF-10.17): off unless asked, kept
/// when not given again, saved with the document, and the drape still drawn in it.
#[test]
fn a_fabric_photo_can_be_evened_and_made_seamless() {
    let mut s = skirt();
    let dir = std::env::temp_dir().join(format!("pf-fabric-even-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("stripes.png");
    std::fs::write(&path, stripes_png(4, 10)).unwrap();
    let photo = s.execute("file.place", &json!({"path": path, "link": false, "at": [0, 2000]})).unwrap()["ids"][0].clone();
    std::fs::remove_dir_all(&dir).ok();
    let f = s.execute("fabric.set", &json!({"id": photo, "width": 200})).unwrap();
    assert_eq!((f["even"].as_bool(), f["seamless"].as_bool()), (Some(false), Some(false)), "{f}");
    let f = s.execute("fabric.set", &json!({"id": photo, "even": true, "seamless": true})).unwrap();
    assert_eq!((f["even"].as_bool(), f["seamless"].as_bool()), (Some(true), Some(true)), "{f}");
    let f = s.execute("fabric.set", &json!({"id": photo, "turns": 0})).unwrap();
    assert_eq!((f["even"].as_bool(), f["seamless"].as_bool(), f["width"].as_f64()), (Some(true), Some(true), Some(200.0)), "{f}");
    let e = s.execute("fabric.set", &json!({"id": photo, "even": "yes"})).unwrap_err().to_string();
    assert!(e.contains("even: true or false"), "{e}");
    let tag =
        serde_json::to_string(s.doc().unwrap().doc.node(NodeId(photo.as_u64().unwrap())).unwrap().extras().pattern.as_deref().unwrap()).unwrap();
    assert!(tag.contains("\"even\":true") && tag.contains("\"seamless\":true"), "{tag}");
    let r = s.execute("drape.make", &json!({"precision": 25, "steps": 5, "origin": [0, 0]})).unwrap();
    assert!(r["fabric"].as_u64().is_some(), "{r}");
}

/// The drape as glTF in its fabric photo (PF-10.18): the cloth a part "Drape: Main" with the photo
/// as its texture (the swatch the drape view uses: the photo's own 80 × 30 pixels, coarser than
/// 1 mm a pixel, so not enlarged, then each side the nearest power of two, 64 × 32), its places on it in photos across (the two panels' 1700 mm on
/// the pattern 8.5 photos of 200 mm); `colour: "fit"` keeps the fit colours; another colour is
/// refused.
#[test]
fn the_drape_exports_as_gltf_in_its_fabric_photo() {
    let mut s = skirt();
    let dir = std::env::temp_dir().join(format!("pf-fabric-glb-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("stripes.png");
    std::fs::write(&path, stripes_png(4, 10)).unwrap();
    let photo = s.execute("file.place", &json!({"path": path, "link": false, "at": [0, 2000]})).unwrap()["ids"][0].clone();
    std::fs::remove_dir_all(&dir).ok();
    s.execute("fabric.set", &json!({"id": photo, "width": 200})).unwrap();
    let r = s.execute("drape.make", &json!({"precision": 25, "steps": 5, "format": "glb", "draw": false})).unwrap();
    let bytes = vectorcraft_format::base64_decode(r["dataBase64"].as_str().unwrap()).unwrap();
    if let Ok(out) = std::env::var("PF_FABRIC_GLB") {
        std::fs::write(out, &bytes).unwrap();
    }
    let (gltf, bin) = pf_core::gltf::read_glb(&bytes).unwrap();
    let names: Vec<&str> = gltf["meshes"].as_array().unwrap().iter().map(|m| m["name"].as_str().unwrap()).collect();
    assert_eq!(names, vec!["Avatar", "Drape: Main"]);
    let prim = &gltf["meshes"][1]["primitives"][0];
    assert!(prim["attributes"].get("COLOR_0").is_none());
    let tex = gltf["materials"][prim["material"].as_u64().unwrap() as usize]["pbrMetallicRoughness"]["baseColorTexture"]["index"].as_u64().unwrap();
    let image = &gltf["images"][gltf["textures"][tex as usize]["source"].as_u64().unwrap() as usize];
    let view = &gltf["bufferViews"][image["bufferView"].as_u64().unwrap() as usize];
    let (o, l) = (view["byteOffset"].as_u64().unwrap() as usize, view["byteLength"].as_u64().unwrap() as usize);
    let png = image::load_from_memory(&bin[o..o + l]).unwrap();
    assert_eq!((png.width(), png.height()), (64, 32));
    // Places on the photo: as many photos across as the pattern is wide over 200 mm.
    let acc = &gltf["accessors"][prim["attributes"]["TEXCOORD_0"].as_u64().unwrap() as usize];
    let v = &gltf["bufferViews"][acc["bufferView"].as_u64().unwrap() as usize];
    let (o, l) = (v["byteOffset"].as_u64().unwrap() as usize, v["byteLength"].as_u64().unwrap() as usize);
    let uv: Vec<f32> = bin[o..o + l].chunks(4).map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]])).collect();
    let us: Vec<f32> = uv.iter().step_by(2).copied().collect();
    let span = us.iter().copied().fold(f32::NEG_INFINITY, f32::max) - us.iter().copied().fold(f32::INFINITY, f32::min);
    // The front panel spans 0 to 800 mm across, the back 900 to 1700: 1700 mm, 8.5 photos.
    assert!((span - 8.5).abs() < 0.3, "{span}");
    // The fit colours on asking.
    let r = s.execute("drape.make", &json!({"precision": 25, "steps": 5, "format": "glb", "draw": false, "colour": "fit"})).unwrap();
    let (gltf, _) = pf_core::gltf::read_glb(&vectorcraft_format::base64_decode(r["dataBase64"].as_str().unwrap()).unwrap()).unwrap();
    assert_eq!(gltf["meshes"][1]["name"], "Drape");
    assert!(gltf["meshes"][1]["primitives"][0]["attributes"].get("COLOR_0").is_some() && gltf.get("images").is_none());
    assert!(s.execute("drape.make", &json!({"steps": 5, "format": "glb", "draw": false, "colour": "red"})).is_err());
}
