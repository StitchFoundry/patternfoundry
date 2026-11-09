use super::*;

fn square(x0: f64, y0: f64, side: f64) -> Vec<Mm> {
    vec![[x0, y0], [x0 + side, y0], [x0 + side, y0 + side], [x0, y0 + side]]
}

fn style(version: Version) -> Style {
    let piece = Piece {
        name: "Test Square".into(),
        quantity: (1, 1),
        material: "Main".into(),
        boundary: square(0.0, 50.0, 120.0),
        sew_line: square(10.0, 60.0, 100.0),
        notches: vec![
            Notch { at: [60.0, 50.0], angle: 90.0, depth: 6.0, width: 0.0, shape: NotchShape::Plain },
            Notch { at: [120.0, 110.0], angle: 180.0, depth: 6.0, width: 3.0, shape: NotchShape::T },
            Notch { at: [40.0, 170.0], angle: -90.0, depth: 6.0, width: 3.0, shape: NotchShape::Castle },
            Notch { at: [0.0, 110.0], angle: 0.0, depth: 6.0, width: 3.0, shape: NotchShape::U },
        ],
        grain: Some([[60.0, 70.0], [60.0, 150.0]]),
        mirror: Some([[0.0, 50.0], [0.0, 170.0]]),
        internal: vec![vec![[20.0, 110.0], [100.0, 110.0]]],
        drills: vec![Drill { at: [40.0, 90.0], diameter: 3.0 }, Drill { at: [80.0, 90.0], diameter: 0.0 }],
        text_at: [20.0, 100.0],
    };
    Style {
        name: "Test".into(),
        author: "StitchFoundry;PatternFoundry;0.1".into(),
        date: "21-10-2026".into(),
        time: "01:30".into(),
        sample_size: "M".into(),
        tolerance: 0.02,
        version,
        pieces: vec![piece],
    }
}

/// The entities of type `kind` on `layer` inside the block `name`.
fn block_entities(s: &str, name: &str, kind: &str, layer: &str) -> Vec<Vec<Tag>> {
    let tags = pf_dxf::parse(s.as_bytes()).unwrap();
    let blocks = pf_dxf::sections(&tags).into_iter().find(|s| s.name == "BLOCKS").unwrap();
    let mut out = Vec::new();
    let mut inside = false;
    for (k, e) in pf_dxf::records(&blocks.tags) {
        match k.as_str() {
            "BLOCK" => inside = e.iter().any(|t| t.code == 2 && t.str() == name),
            "ENDBLK" => inside = false,
            _ if inside && k == kind && e.iter().any(|t| t.code == 8 && t.str() == layer) => out.push(e),
            _ => {}
        }
    }
    out
}

fn values(e: &[Tag], code: i32) -> Vec<f64> {
    e.iter().filter(|t| t.code == code).map(Tag::f64).collect()
}

fn span(xs: &[f64]) -> f64 {
    xs.iter().cloned().fold(f64::NEG_INFINITY, f64::max) - xs.iter().cloned().fold(f64::INFINITY, f64::min)
}

#[test]
fn a_piece_is_a_block_with_its_cutting_line_sew_line_and_marks_on_their_layers() {
    for version in [Version::R13, Version::R12] {
        let s = write(&style(version)).unwrap();
        assert!(s.contains(version.acadver()));
        let verts = |layer: &str| -> (Vec<f64>, Vec<f64>) {
            let v = block_entities(&s, "TEST_SQUARE", "VERTEX", layer);
            (v.iter().flat_map(|e| values(e, 10)).collect(), v.iter().flat_map(|e| values(e, 20)).collect())
        };
        // True scale: the 10 cm sew line is 100 mm across in the file, the cutting line 120.
        let (sx, sy) = verts("14");
        assert_eq!((sx.len(), span(&sx), span(&sy)), (4, 100.0, 100.0), "{version:?}");
        let (bx, by) = verts("1");
        assert_eq!((span(&bx), span(&by)), (120.0, 120.0));
        // The validation curves copy them.
        assert_eq!(verts("84"), (bx, by));
        assert_eq!(verts("87"), (sx, sy));
        // Four turn points on each line, no curve points on a square.
        assert_eq!(block_entities(&s, "TEST_SQUARE", "POINT", "2").len(), 8);
        assert!(block_entities(&s, "TEST_SQUARE", "POINT", "3").is_empty());
        // The notch: a point on the cutting line, 6 deep, cut at 90°.
        let notch = &block_entities(&s, "TEST_SQUARE", "POINT", "4")[0];
        assert_eq!((values(notch, 10), values(notch, 30), values(notch, 50)), (vec![60.0], vec![6.0], vec![90.0]));
        // The other shapes on their own layers; a T notch's point is at its depth, 6 mm in.
        let t = &block_entities(&s, "TEST_SQUARE", "POINT", "80")[0];
        assert_eq!((values(t, 10), values(t, 39)), (vec![114.0], vec![3.0]));
        assert_eq!(block_entities(&s, "TEST_SQUARE", "POINT", "81").len(), 1);
        assert_eq!(block_entities(&s, "TEST_SQUARE", "POINT", "83").len(), 1);
        assert_eq!(block_entities(&s, "TEST_SQUARE", "LINE", "7").len(), 1);
        let m = block_entities(&s, "TEST_SQUARE", "LINE", "6");
        assert_eq!((m.len(), values(&m[0], 10), values(&m[0], 21)), (1, vec![0.0], vec![170.0]), "the mirror line");
        assert_eq!(block_entities(&s, "TEST_SQUARE", "POLYLINE", "8").len(), 1);
        // The drill holes: points on layer 13, the punch's diameter their z.
        let drills = block_entities(&s, "TEST_SQUARE", "POINT", "13");
        assert_eq!(
            drills.iter().map(|e| (values(e, 10)[0], values(e, 20)[0], values(e, 30)[0])).collect::<Vec<_>>(),
            [(40.0, 90.0, 3.0), (80.0, 90.0, 0.0)]
        );
        // The piece system text in the block, the style system text outside it.
        let pst: Vec<String> = block_entities(&s, "TEST_SQUARE", "TEXT", "1").iter().map(|e| text_of(e)).collect();
        assert_eq!(pst, ["Piece Name: Test Square", "Quantity: 1,1", "Material: Main"]);
        for line in ["Style Name: Test", "Units: METRIC", "Creation Date: 21-10-2026", "Sample Size: M", "Curve Tolerance: 0.02"] {
            assert!(s.contains(&format!("\r\n{line}\r\n")), "{line}");
        }
        assert_eq!(s.matches("\r\nINSERT\r\n").count(), 1);
        assert!(s.ends_with("EOF\r\n"));
    }
}

#[test]
fn a_file_reads_back_as_the_style_it_was_written_from() {
    for version in [Version::R13, Version::R12] {
        let st = style(version);
        let back = read(write(&st).unwrap().as_bytes()).unwrap();
        assert_eq!(back, st, "{version:?}");
    }
}

#[test]
fn a_piece_placed_by_its_insert_and_drawn_with_a_lightweight_polyline_is_read_there() {
    let text = "0\nSECTION\n2\nBLOCKS\n0\nBLOCK\n8\n0\n2\nP\n0\nTEXT\n8\n1\n10\n1\n20\n1\n1\nPiece Name: Yoke\n0\nLWPOLYLINE\n8\n1\n70\n1\n10\n0\n20\n0\n10\n10\n20\n0\n10\n10\n20\n10\n0\nENDBLK\n8\n0\n0\nENDSEC\n0\nSECTION\n2\nENTITIES\n0\nINSERT\n8\n1\n2\nP\n10\n100\n20\n50\n0\nENDSEC\n0\nEOF\n";
    let st = read(text.as_bytes()).unwrap();
    assert_eq!(st.pieces.len(), 1);
    assert_eq!(st.pieces[0].name, "Yoke");
    assert_eq!(st.pieces[0].boundary, vec![[100.0, 50.0], [110.0, 50.0], [110.0, 60.0]]);
}

#[test]
fn names_are_unique_block_names_and_bad_input_is_refused() {
    let mut st = style(Version::R13);
    let mut second = st.pieces[0].clone();
    second.name = "test square".into();
    st.pieces.push(second);
    let s = write(&st).unwrap();
    assert!(s.contains("\r\nTEST_SQUARE\r\n") && s.contains("\r\nTEST_SQUARE_2\r\n"));
    assert_eq!(write(&Style { pieces: vec![], ..style(Version::R13) }), Err(AamaError::NoPieces));
    let mut bad = style(Version::R13);
    bad.pieces[0].boundary.truncate(2);
    assert!(matches!(write(&bad), Err(AamaError::NoOutline(_))));
    assert!(read(b"not a dxf").is_err());
    assert_eq!(read(b"0\nSECTION\n2\nENTITIES\n0\nENDSEC\n0\nEOF\n"), Err(AamaError::NoPieces));
}

#[test]
fn corners_are_turn_points_and_bends_curve_points() {
    // A quarter circle in 10° steps closed by two straight edges.
    let mut poly: Vec<Mm> = (0..=9)
        .map(|k| {
            let a = (f64::from(k) * 10.0).to_radians();
            [100.0 * a.cos(), 100.0 * a.sin()]
        })
        .collect();
    poly.push([0.0, 0.0]);
    let (turns, curves) = turn_and_curve_points(&poly);
    assert_eq!((turns.len(), curves.len()), (3, 8));
}

#[test]
fn each_notch_shape_is_drawn_as_its_cut() {
    let n = |shape, width| Notch { at: [0.0, 0.0], angle: 90.0, depth: 6.0, width, shape };
    let near = |a: Mm, b: Mm| (a[0] - b[0]).abs() < 1e-9 && (a[1] - b[1]).abs() < 1e-9;
    // Cut upward (y up) from the origin; with y down the same angle cuts toward -y.
    let slit = n(NotchShape::Plain, 0.0).lines(false);
    assert!(near(slit[0][1], [0.0, 6.0]));
    assert!(near(n(NotchShape::Plain, 0.0).lines(true)[0][1], [0.0, -6.0]));
    let v = n(NotchShape::Plain, 4.0).lines(false);
    assert!(near(v[0][0], [-2.0, 0.0]) && near(v[0][1], [0.0, 6.0]) && near(v[0][2], [2.0, 0.0]), "{v:?}");
    let t = n(NotchShape::T, 3.0).lines(false);
    assert!(t.len() == 2 && near(t[1][0], [-1.5, 6.0]) && near(t[1][1], [1.5, 6.0]), "{t:?}");
    let c = n(NotchShape::Castle, 3.0).lines(false);
    assert!(near(c[0][1], [-1.5, 6.0]) && near(c[0][2], [1.5, 6.0]), "{c:?}");
    let u = &n(NotchShape::U, 3.0).lines(false)[0];
    let top = u.iter().map(|p| p[1]).fold(f64::NEG_INFINITY, f64::max);
    assert!((top - 6.0).abs() < 1e-9 && near(u[0], [-1.5, 0.0]) && near(u[u.len() - 1], [1.5, 0.0]), "{u:?}");
}
