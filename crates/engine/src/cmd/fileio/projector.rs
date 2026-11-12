//! PDF for projector (`document.export {format: projector}`, PF-7.3): the document's pattern
//! pieces on one page, at 100%, for projecting onto fabric on the cutting table. As projector
//! sewists describe such files (one page with every piece and room round it, thick solid cutting
//! lines, bigger labels, a 10 cm and a 4 in calibration grid on layers of their own:
//! https://www.craftstorming.com/2020/05/tips-for-using-pdf-sewing-patterns-on-a-projector),
//! with a 10 cm test square above the pieces to check the projection by, the pieces set side by
//! side so none overlaps another (blocks are drafted touching), and optionally white
//! lines on black (easier to see on fabric and less glare) and the page mirrored (for cutting
//! with the fabric's wrong side up). A piece cut on the fold is shown opened out, whole, for fabric
//! laid out single (PF-7.4), or as the half with its fold line, for fabric folded.

use std::sync::Arc;

use serde_json::{Value, json};
use vectorcraft_color::{Color, Paint};
use vectorcraft_doc::{Appearance, CharStyle, Dash, Document, Node, NodeKind, TextObject};
use vectorcraft_geom::{Affine, PathData, Point, Rect, SubPath};

use super::super::*;
use super::{Encoded, FormatOption};
use pf_core::{mm_to_pt, pt_to_mm};

const C: &str = "document.export";
/// The test square's side and the grid's: 10 cm.
const SQUARE_MM: f64 = 100.0;
/// The inch grid's spacing: 4 in.
const INCH_GRID_MM: f64 = 4.0 * 25.4;
/// The piece names' type size, mm.
const TEXT_MM: f64 = 10.0;

/// The projector PDF's options `document.formats` lists.
pub(super) const OPTIONS: &[FormatOption] = &[
    FormatOption {
        name: "size",
        ty: "string",
        default: "\"\"",
        description: "write the pieces graded to this size of the size chart or size range (default: as drawn)",
    },
    FormatOption { name: "margin", ty: "number", default: "25", description: "room round the pieces, mm" },
    FormatOption {
        name: "lineWidth",
        ty: "number",
        default: "3",
        description: "the cutting lines' and notches' weight, pt (thin lines vanish when projected)",
    },
    FormatOption {
        name: "layout",
        ty: "string",
        default: "row",
        description: "row: the pieces side by side with the margin between them; asDrawn: where they are in the document",
    },
    FormatOption {
        name: "openFolds",
        ty: "boolean",
        default: "true",
        description: "pieces cut on the fold opened out, whole (false: the half, its fold line dashed, for folded fabric)",
    },
    FormatOption { name: "invert", ty: "boolean", default: "false", description: "white lines on a black page" },
    FormatOption {
        name: "mirror",
        ty: "boolean",
        default: "false",
        description: "the page mirrored left to right (to cut with the fabric's wrong side up); the type stays readable",
    },
];

fn number(p: &Value, key: &str, default: f64, min: f64, max: f64) -> Result<f64> {
    match p.get(key) {
        None | Some(Value::Null) => Ok(default),
        Some(v) => match v.as_f64() {
            Some(x) if (min..=max).contains(&x) => Ok(x),
            _ => Err(bad(C, format!("{key}: a number from {min} to {max}"))),
        },
    }
}

/// The art of the projector page: layers, with their visibility, bottom first.
struct Page {
    doc: Document,
    ink: Paint,
}

impl Page {
    fn node(&mut self, kind: NodeKind, look: Appearance) -> Arc<Node> {
        let mut n = Node::new(self.doc.alloc_id(), kind);
        n.appearance = look.into();
        Arc::new(n)
    }

    fn path(&mut self, subs: Vec<SubPath>, width: f64, dash: Option<Vec<f64>>, ink: Option<Paint>) -> Arc<Node> {
        let mut look = Appearance::basic(Paint::None, ink.unwrap_or_else(|| self.ink.clone()), width);
        if let (Some(d), Some(s)) = (dash, look.stroke_mut()) {
            s.dash = Some(Dash { pattern: d, ..Default::default() });
        }
        self.node(NodeKind::Path { path: PathData::new(subs), rule: Default::default(), live: None, clipping: false, guide: false }, look)
    }

    /// Type whose top left is at `at` (pt).
    fn text(&mut self, at: Point, text: &str, size: f64) -> Arc<Node> {
        let style = CharStyle { size, fill: self.ink.clone(), ..Default::default() };
        let mut t = TextObject::point(Point::ZERO, text, style);
        t.xf = Affine::translate((at.x, at.y + size * 0.8));
        let ink = self.ink.clone();
        self.node(NodeKind::Text(Box::new(t)), Appearance::basic(ink, Paint::None, 0.0))
    }

    fn layer(&mut self, name: &str, visible: bool, children: Vec<Arc<Node>>) {
        let color = vectorcraft_doc::LayerColor::Preset(self.doc.layers.len() as u8);
        let mut l = Node::layer(self.doc.alloc_id(), name, color);
        l.visible = visible;
        if let Some(c) = l.children_mut() {
            *c = children;
        }
        self.doc.layers.push(Arc::new(l));
    }
}

fn poly(pts: &[Point], closed: bool) -> SubPath {
    SubPath::polyline(pts, closed)
}

/// Lines every `step` pt across `area`, from its top left corner.
fn grid(area: Rect, step: f64) -> Vec<SubPath> {
    let mut subs = Vec::new();
    let mut x = area.x0;
    while x <= area.x1 + 1e-6 {
        subs.push(poly(&[Point::new(x, area.y0), Point::new(x, area.y1)], false));
        x += step;
    }
    let mut y = area.y0;
    while y <= area.y1 + 1e-6 {
        subs.push(poly(&[Point::new(area.x0, y), Point::new(area.x1, y)], false));
        y += step;
    }
    subs
}

/// `[x0, y0, x1, y1]` round `pts`.
fn bounds<'a>(pts: impl Iterator<Item = &'a [f64; 2]>) -> [f64; 4] {
    pts.fold([f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY], |[x0, y0, x1, y1], q| {
        [x0.min(q[0]), y0.min(q[1]), x1.max(q[0]), y1.max(q[1])]
    })
}

/// `piece` moved by `(dx, dy)`.
fn shift(piece: &mut pf_aama::Piece, dx: f64, dy: f64) {
    let mv = |q: &mut [f64; 2]| *q = [q[0] + dx, q[1] + dy];
    piece.boundary.iter_mut().chain(piece.sew_line.iter_mut()).chain(piece.internal.iter_mut().flatten()).for_each(mv);
    piece.notches.iter_mut().for_each(|n| mv(&mut n.at));
    piece.drills.iter_mut().for_each(|d| mv(&mut d.at));
    piece.grain.iter_mut().flatten().for_each(mv);
    piece.mirror.iter_mut().flatten().for_each(mv);
    mv(&mut piece.text_at);
}

pub(super) fn encode(doc: &Document, p: &Value) -> Result<Encoded> {
    let margin = mm_to_pt(number(p, "margin", 25.0, 0.0, 1000.0)?);
    let line = number(p, "lineWidth", 3.0, 0.1, 20.0)?;
    let invert = p.get("invert").and_then(Value::as_bool).unwrap_or(false);
    let mirror = p.get("mirror").and_then(Value::as_bool).unwrap_or(false);
    let open = p.get("openFolds").and_then(Value::as_bool).unwrap_or(true);
    let row = match p.get("layout").and_then(Value::as_str) {
        None | Some("row") => true,
        Some("asDrawn") => false,
        Some(other) => return Err(bad(C, format!("layout `{other}`: row or asDrawn"))),
    };
    // The pieces in points, y down (as the document is).
    let pieces = super::aama::pieces(doc)?;
    if pieces.is_empty() {
        return Err(bad(C, "no pattern pieces to export: make the outlines pattern pieces first (Pattern › Make Pattern Piece)"));
    }
    let mut pieces: Vec<pf_aama::Piece> = if open { pieces.iter().map(super::aama::opened).collect() } else { pieces };
    if row {
        // Side by side, tops level, the margin between them, in the document's order.
        let mut x = 0.0;
        for piece in &mut pieces {
            let b = bounds(piece.boundary.iter());
            shift(piece, x - b[0], -b[1]);
            x += b[2] - b[0] + margin;
        }
    }
    let [x0, y0, x1, y1] = bounds(pieces.iter().flat_map(|p| &p.boundary));
    // The test square sits in a band above the pieces.
    let square = mm_to_pt(SQUARE_MM);
    let top = margin + square + margin;
    let (w, h) = ((x1 - x0).max(square) + 2.0 * margin, top + (y1 - y0) + margin);
    // Into the page; mirrored left to right when asked.
    let at = |q: [f64; 2]| {
        let x = q[0] - x0 + margin;
        Point::new(if mirror { w - x } else { x }, q[1] - y0 + top)
    };
    let mut d = Document::new(w, h);
    d.title = if doc.title.trim().is_empty() { "Pattern for projector".into() } else { format!("{} (projector)", doc.title) };
    d.layers.clear();
    d.artboards[0].name = "Projector".into();
    let (ink, paper) = if invert { (Color::WHITE, Color::BLACK) } else { (Color::BLACK, Color::WHITE) };
    let mut page = Page { doc: d, ink: Paint::solid(ink) };
    if invert {
        let all = Rect::new(0.0, 0.0, w, h);
        let corners = [Point::new(all.x0, all.y0), Point::new(all.x1, all.y0), Point::new(all.x1, all.y1), Point::new(all.x0, all.y1)];
        let kind =
            NodeKind::Path { path: PathData::new(vec![poly(&corners, true)]), rule: Default::default(), live: None, clipping: false, guide: false };
        let bg = page.node(kind, Appearance::basic(Paint::solid(paper), Paint::None, 0.0));
        page.layer("Background", true, vec![bg]);
    }
    // The grids, over the whole page from its top left corner, grey.
    let grey = Paint::solid(Color::rgb(0.5, 0.5, 0.5));
    let area = Rect::new(0.0, 0.0, w, h);
    let cm = page.path(grid(area, square), 0.75, None, Some(grey.clone()));
    page.layer("10 cm grid", true, vec![cm]);
    let inch = page.path(grid(area, mm_to_pt(INCH_GRID_MM)), 0.75, Some(vec![6.0, 4.0]), Some(grey));
    page.layer("4 in grid", false, vec![inch]);
    // The test square, a path of its own so a reader measures it alone.
    let s0 = Point::new(if mirror { w - margin - square } else { margin }, margin);
    let sq = [s0, Point::new(s0.x + square, s0.y), Point::new(s0.x + square, s0.y + square), Point::new(s0.x, s0.y + square)];
    let sq = page.path(vec![poly(&sq, true)], 1.0, None, None);
    let label = page.text(Point::new(s0.x + mm_to_pt(5.0), s0.y + mm_to_pt(5.0)), "10 cm", mm_to_pt(TEXT_MM));
    page.layer("Test square", true, vec![sq, label]);
    let mut art = Vec::new();
    for piece in &pieces {
        let pts: Vec<Point> = piece.boundary.iter().map(|q| at(*q)).collect();
        art.push(page.path(vec![poly(&pts, true)], line, None, None));
        if piece.sew_line.len() >= 3 {
            let pts: Vec<Point> = piece.sew_line.iter().map(|q| at(*q)).collect();
            art.push(page.path(vec![poly(&pts, true)], 1.0, Some(vec![6.0, 3.0]), None));
        }
        // Each notch as its cut; the document's y runs down.
        for l in piece.notches.iter().flat_map(|nt| nt.lines(true)) {
            let pts: Vec<Point> = l.iter().map(|q| at(*q)).collect();
            art.push(page.path(vec![poly(&pts, false)], line, None, None));
        }
        if let Some([a, b]) = piece.grain {
            art.push(page.path(vec![poly(&[at(a), at(b)], false)], 2.0, None, None));
        }
        // The half of a piece cut on the fold: the fold, dashed, to lay on the fabric's fold.
        if let Some([a, b]) = piece.mirror {
            art.push(page.path(vec![poly(&[at(a), at(b)], false)], line, Some(vec![12.0, 6.0]), None));
        }
        for l in piece.internal.iter().filter(|l| l.len() >= 2) {
            let pts: Vec<Point> = l.iter().map(|q| at(*q)).collect();
            art.push(page.path(vec![poly(&pts, false)], 1.0, None, None));
        }
        // Drill holes as their crosses (and punch circles): the diameter is in points here.
        for dr in &piece.drills {
            let d = pf_core::Drill { at: pf_core::Pt::new(dr.at[0], dr.at[1]), diameter: pt_to_mm(dr.diameter) };
            let subs =
                pf_core::marks::drill(&d, mm_to_pt(1.0)).iter().map(|l| poly(&l.iter().map(|q| at([q.x, q.y])).collect::<Vec<_>>(), false)).collect();
            art.push(page.path(subs, line, None, None));
        }
        let cut = piece.quantity.0 + piece.quantity.1;
        let mut text = format!("{}\nCut {cut}", piece.name);
        if piece.mirror.is_some() {
            text.push_str(" on fold");
        }
        if piece.quantity.1 > 0 {
            text.push_str(" (mirrored pairs)");
        }
        if !piece.material.is_empty() {
            text.push_str(&format!(" · {}", piece.material));
        }
        // Designs to embroider or print on the piece, one line each.
        for note in &piece.notes {
            text.push('\n');
            text.push_str(note);
        }
        let t = at(piece.text_at);
        art.push(page.text(Point::new(t.x + mm_to_pt(3.0), t.y), &text, mm_to_pt(TEXT_MM)));
    }
    page.layer("Pieces", true, art);
    let settings = json!({"createLayers": true, "preserveEditing": false, "compatibility": "1.7"});
    let (bytes, warnings) = super::pdf::encode(C, &page.doc, &settings)?;
    Ok(Encoded { warnings, ..Encoded::one(bytes) })
}
