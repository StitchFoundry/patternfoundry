//! True scale (PatternFoundry: "a pattern printed at 100% must measure exactly"): an exported SVG
//! gives its size in points, so every SVG reader (which takes a unitless size as CSS pixels, 96
//! per inch) shows a 10 cm square 10 cm across, and it opens again at its size.
// Integration tests: unwrapping and panicking on failure is fine here, unlike in shipped code (AGENTS.md › Robustness).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use vectorcraft_doc::{Appearance, Document, Node};
use vectorcraft_geom::{Rect, shapes};
use vectorcraft_svg::{ExportOptions, export, import};

/// 100 mm in points.
const CM10: f64 = 100.0 / 25.4 * 72.0;

#[test]
fn the_10_cm_test_square_measures_10_cm_in_an_svg_reader() {
    let mut d = Document::new(CM10, CM10);
    let n = Node::path(d.alloc_id(), shapes::rectangle(Rect::new(0.0, 0.0, CM10, CM10)), Appearance::default_art());
    let l = d.layers[0].id;
    d.insert(Some(l), 0, n).unwrap();
    let svg = export(&d, &ExportOptions::default());
    // A reader at its standard 96 pixels per inch.
    let tree = resvg::usvg::Tree::from_str(&svg, &resvg::usvg::Options::default()).unwrap();
    let mm = |px: f32| f64::from(px) / 96.0 * 25.4;
    let (w, h) = (mm(tree.size().width()), mm(tree.size().height()));
    assert!((w - 100.0).abs() < 0.01 && (h - 100.0).abs() < 0.01, "{w} × {h} mm: {svg}");
    // The square fills the page: drawn 100 mm across.
    let b = tree.root().abs_bounding_box().unwrap();
    assert!((mm(b.width()) - 100.0).abs() < 0.01, "{}", mm(b.width()));
    // Opened again, it is the same 10 cm.
    let back = import(&svg).unwrap();
    let r = back.artboards[0].rect;
    assert!((r.width() - CM10).abs() < 0.01 && (r.height() - CM10).abs() < 0.01, "{r:?}");
}
