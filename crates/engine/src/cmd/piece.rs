//! Pattern pieces (PF-1): a closed path becomes a sewing-pattern piece.
//!
//! A piece is a group holding the path the user drew, its **seam line**, with the piece's data
//! ([`pf_core::Piece`]: name, cut count, fabric, seam allowance per edge, corner styles, notches,
//! grainline) on the group. The cutting line, the notches, the grainline and the label are
//! generated from the seam line as locked children, so they print and export like any art, and
//! [`refresh`] keeps them in step whenever the seam line or the piece's data changes (as linked
//! dimensions follow the art they measure). The seam line's segments are the piece's edges, numbered
//! from 0 in drawing order.
//!
//! Lengths come and go in millimetres (a number; or a string with a unit, `"5/8 in"`); the
//! document is in points.
use super::*;
use pf_core::{Corner, Mark, Notch, NotchKind, Outline, PatternTag, Piece, Pt, mm_to_pt, pt_to_mm};
use serde_json::{Value, json};
use std::sync::Arc;
use vectorcraft_color::{Color, Paint};
use vectorcraft_doc::{Appearance, CharStyle, Document, Node, NodeId, NodeKind, TextObject};
use vectorcraft_geom::{Affine, BezPath, PathData, Point, SubPath};

/// Most pieces one command makes or changes.
const MAX_PIECES: usize = 1000;
/// How closely curves of the seam line are followed, in points (a fiftieth of a millimetre is
/// below what a printer or a cutter can show).
pub(crate) const FLATTEN_PT: f64 = 0.05;
/// The label's type size, in points.
const LABEL_PT: f64 = 10.0;

mod placements;

pub fn specs() -> Vec<CommandSpec> {
    let mut v = own_specs();
    v.extend(placements::specs());
    v
}

fn own_specs() -> Vec<CommandSpec> {
    vec![
        cmd!(
            "piece.make",
            "Make Pattern Piece",
            ["Pattern"],
            None,
            "{ids? (default: the selection), allowance?: mm (10; or a length with its unit, \"5/8 in\"), name?} each closed path becomes a pattern piece: a group of its seam line (the path) with its cutting line, grainline and label generated → {ids}",
            has_selection,
            make
        ),
        cmd!(
            "piece.release",
            "Release Pattern Piece",
            ["Pattern"],
            None,
            "{ids? (default: the selected pieces)} each piece goes back to its seam line, a plain path; the generated marks are deleted → {ids}",
            has_selection,
            release
        ),
        cmd!(
            "piece.set",
            "Piece Options",
            [],
            None,
            "{id? (default: the selected piece), name?, cut?: 1..999, mirror?: bool (cut in mirror-image pairs), fabric?, allowance?: mm (every edge without its own), edges?: {\"<edge>\": mm | null (back to the piece's allowance)}, corners?: {\"<corner>\": \"mitre\" | \"square\"} (corner i is where edge i-1 meets edge i), grainline?: {from: [x, y], to: [x, y]} (document points) | \"auto\" (vertical through the middle), fold?: edge | \"auto\" (the longest straight edge) | null, foldAt?: [x, y] (instead: the straight edge nearest this document point), edgeAllowance?: mm | null (the allowance of the edge nearest the point at), at?: [x, y] (document points, with edgeAllowance) (the straight edge cut on the fold: no allowance, laid against the fabric's fold or opened out by marker.make)} → piece.info",
            has_doc,
            set
        ),
        cmd!(
            "piece.addNotch",
            "Add Notch",
            [],
            None,
            "{id?, edge: 0.., at?: 0..1 (0.5; along the edge by length) | point: [x, y] (instead: document points on the seam line or in its allowance, as a click on the canvas: the nearest edge there), kind?: \"slit\" (default) | \"double\" | \"v\" | \"t\" | \"castle\" | \"u\"} → piece.info",
            has_doc,
            add_notch
        ),
        cmd!(
            "piece.setNotch",
            "Notch Shape",
            [],
            None,
            "{id?, index: the notch's place in piece.info's notches, kind: \"slit\" | \"double\" | \"v\" | \"t\" | \"castle\" | \"u\"} → piece.info",
            has_doc,
            set_notch
        ),
        cmd!(
            "piece.removeNotch",
            "Remove Notch",
            [],
            None,
            "{id?, index: the notch's place in piece.info's notches} → piece.info",
            has_doc,
            remove_notch
        ),
        cmd!(
            "piece.addDrill",
            "Add Drill Hole",
            [],
            None,
            "{id?, point: [x, y] (document points, inside the piece's cutting line), diameter?: mm (0: a mark only; up to 50)} a drill hole, the punch mark for a dart's point, a pocket's corners or a button, drawn as a cross and written to DXF-AAMA on layer 13 → piece.info",
            has_doc,
            add_drill
        ),
        cmd!(
            "piece.sew",
            "Sew Seams",
            [],
            None,
            "{a: {id (a piece, or its seam line), from: anchor, to: anchor}, b: {id, from, to} (the two runs of seam line that are sewn together, each from anchor from forward to anchor to along its seam line; a and b the same run on a piece cut as a mirror-image pair (cut 2, mirror): a centre seam, that run of one copy sewn to the other's, as trousers' centre front and back are; when draped, their ends are matched by height on the pattern, as the pieces are drawn upright), label? (default the next free letter)} record that the two runs are sewn together (for the 3D fit preview): each piece keeps its run with the label, and it follows the seam line when anchors are added → {label, a (mm), b (mm), difference (mm, a less b)}",
            has_doc,
            sew
        ),
        cmd!(
            "piece.unsew",
            "Unsew Seams",
            [],
            None,
            "{label} forget the seams sewn under that label (piece.seams lists them) → {label, runs (how many were removed)}",
            has_doc,
            unsew
        ),
        cmd!(
            query "piece.seams",
            "Sewn Seams",
            [],
            None,
            "{} every pair of runs sewn together (piece.sew), by label → {seams: [{label, runs: [{piece, name, from, to, mm}], difference (mm, the first less the second)}]}",
            has_doc,
            seams
        ),
        cmd!(
            "piece.removeDrill",
            "Remove Drill Hole",
            [],
            None,
            "{id?, index: the drill hole's place in piece.info's drills | point: [x, y] (instead: the drill hole nearest this document point, within 5 mm)} → piece.info",
            has_doc,
            remove_drill
        ),
        cmd!(
            query "piece.info",
            "Piece Info",
            [],
            None,
            "{id? (default: the selected piece)} → {id, seam (the seam line's id), name, cut, mirror, fabric, allowance, edges, corners, notches, grainline, drills, edgeCount, seamLength (mm), area (cm², inside the seam line), problem (why there is no cutting line, if there is none)}",
            has_doc,
            info
        ),
        cmd!(query "piece.list", "Pattern Pieces", [], None, "{} the document's pieces, in paint order → {pieces: [{id, name, cut, mirror, fabric}]}", has_doc, list),
        cmd!(
            query "piece.outlines",
            "Piece Outlines",
            [],
            None,
            "{openFolds?: bool (true: pieces cut on the fold opened out, whole), size? (a size of the size range or chart)} every piece's lines in millimetres, y down, where they are in the document, as the projector view draws them → {pieces: [{name, text (the label: name and how to cut), cut: [[x, y]] (closed), seam: [[x, y]] (closed), notches: [[[x, y], [x, y]]], grain: [[x, y], [x, y]] | null, fold: [[x, y], [x, y]] | null (the half's fold line), lines: [[[x, y]]] (drawn inside), drills: [{at: [x, y], diameter: mm}], label: [x, y]}], bounds: [x0, y0, x1, y1]}",
            has_doc,
            outlines
        ),
    ]
}

fn err(cmd: &str, e: impl ToString) -> EngineError {
    bad(cmd, e.to_string())
}

/// The piece on `n`, if it is a piece's group.
pub(crate) fn piece_on(n: &Node) -> Option<&Piece> {
    n.extras().pattern.as_deref().and_then(PatternTag::piece)
}

/// Which part of a piece `n` draws, if it is one.
pub(crate) fn mark_of(n: &Node) -> Option<Mark> {
    n.extras().pattern.as_deref().and_then(PatternTag::mark)
}

/// The piece group `id` is or is in (one of its parts).
pub(crate) fn piece_group(doc: &Document, id: NodeId) -> Option<NodeId> {
    let n = doc.node(id)?;
    if piece_on(n).is_some() {
        return Some(id);
    }
    let parent = doc.parent_of(id)?;
    doc.node(parent).and_then(piece_on).map(|_| parent)
}

/// The pieces a command acts on: `ids`/`id` in the params, else the selected pieces (or pieces
/// with a selected part).
pub(crate) fn target_pieces(s: &Session, p: &Value, cmd: &str) -> Result<Vec<NodeId>> {
    let st = s.doc()?;
    let given: Vec<NodeId> = if let Some(id) = id_param(p, "id") {
        vec![id]
    } else if p.get("ids").is_some() {
        checked_ids_param(s, p, "ids", cmd)?
    } else {
        st.selection.in_paint_order(&st.doc)
    };
    let mut out: Vec<NodeId> = Vec::new();
    for id in given {
        if let Some(g) = piece_group(&st.doc, id)
            && !out.contains(&g)
        {
            out.push(g);
        }
    }
    if out.is_empty() {
        return Err(bad(cmd, "select a pattern piece (Pattern › Make Pattern Piece makes one)"));
    }
    if out.len() > MAX_PIECES {
        return Err(bad(cmd, "too many pieces at once"));
    }
    Ok(out)
}

fn one_piece(s: &Session, p: &Value, cmd: &str) -> Result<NodeId> {
    let ids = target_pieces(s, p, cmd)?;
    match ids.as_slice() {
        [one] => Ok(*one),
        _ => Err(bad(cmd, "select one pattern piece")),
    }
}

/// A length param in millimetres: a number (mm) or a string with a unit (`"5/8 in"`, `"1 cm"`).
pub(crate) fn mm_param(v: &Value) -> Option<f64> {
    match v {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => vectorcraft_doc::Unit::Millimeters.parse(s).map(pt_to_mm),
        _ => None,
    }
    .filter(|v| v.is_finite())
}

fn allowance_param(v: &Value, cmd: &str) -> Result<f64> {
    let mm = mm_param(v).ok_or_else(|| bad(cmd, "a seam allowance is a number of mm, or a length with its unit (\"5/8 in\")"))?;
    pf_core::piece::check_allowance(mm).map_err(|e| err(cmd, e))
}

/// The seam line's first closed subpath, flattened into an outline in document points: segment
/// `i` of the subpath is edge `i`. Also the number of edges.
pub(crate) fn outline_of(path: &PathData) -> Option<(Outline, usize)> {
    let sp: &SubPath = path.subpaths.iter().find(|sp| sp.closed)?;
    let edges = sp.segment_count();
    let mut out = Outline::default();
    for i in 0..edges {
        let seg = sp.segment(i);
        out.points.push(Pt::new(seg.p0.x, seg.p0.y));
        out.edge_of.push(i);
        if sp.segment_is_line(i) {
            continue;
        }
        let mut bez = BezPath::new();
        bez.move_to(seg.p0);
        bez.curve_to(seg.p1, seg.p2, seg.p3);
        let mut pts: Vec<Point> = Vec::new();
        vectorcraft_geom::kurbo::flatten(bez, FLATTEN_PT, |el| match el {
            vectorcraft_geom::kurbo::PathEl::LineTo(p) => pts.push(p),
            vectorcraft_geom::kurbo::PathEl::MoveTo(_) | vectorcraft_geom::kurbo::PathEl::ClosePath => {}
            _ => {}
        });
        // The last point is the next segment's start.
        pts.pop();
        for p in pts {
            out.points.push(Pt::new(p.x, p.y));
            out.edge_of.push(i);
        }
        if out.points.len() > pf_core::allowance::MAX_POINTS {
            return None;
        }
    }
    Some((out, edges))
}

fn polyline(points: &[Pt], closed: bool) -> SubPath {
    SubPath::polyline(&points.iter().map(|p| Point::new(p.x, p.y)).collect::<Vec<_>>(), closed)
}

fn black() -> Paint {
    Paint::solid(Color::BLACK)
}

/// The seam line's look: a thin dashed line, no fill.
pub(crate) fn seam_look() -> Appearance {
    let mut a = Appearance::basic(Paint::None, black(), 0.5);
    if let Some(st) = a.stroke_mut() {
        st.dash = Some(vectorcraft_doc::Dash { pattern: vec![4.0, 2.0], ..Default::default() });
    }
    a
}

/// A generated part: locked (it is drawn from the seam line, not edited), tagged with its role.
fn mark_node(id: NodeId, mark: Mark, kind: NodeKind, appearance: Appearance) -> Node {
    let mut n = Node::new(id, kind);
    n.appearance = appearance.into();
    n.locked = true;
    n.name = Some(
        match mark {
            Mark::Seam => "Seam line",
            Mark::Cut => "Cutting line",
            Mark::Notches => "Notches",
            Mark::Grainline => "Grainline",
            Mark::Label => "Label",
            Mark::Drills => "Drill holes",
            Mark::Placements => "Design placements",
        }
        .into(),
    );
    n.extras_mut().pattern = Some(Box::new(PatternTag::Mark(mark)));
    n
}

fn path_kind(subpaths: Vec<SubPath>) -> NodeKind {
    NodeKind::Path { path: PathData::new(subpaths), rule: Default::default(), live: None, clipping: false, guide: false }
}

/// Why `piece` on `seam` has no cutting line, if it has none.
fn problem(seam: Option<&Node>) -> Option<String> {
    let Some(seam) = seam else { return Some("the piece has no seam line".into()) };
    let NodeKind::Path { path, .. } = &seam.kind else { return Some("the seam line is not a path".into()) };
    let Some((outline, _)) = outline_of(path) else { return Some("the seam line is not a closed path".into()) };
    pf_core::cutting_line(&outline, |_| 0.0, |_| Corner::Mitre).err().map(|e| e.to_string())
}

/// A new piece group with id `gid` of the closed path `seam` (its seam line), `piece`'s data and
/// `inside` (drawn inside it, kept as they are). Its parts are drawn by [`rebuild`] once it is in
/// a document.
pub(crate) fn new_piece(gid: NodeId, mut seam: Node, piece: Piece, inside: Vec<Node>) -> Node {
    seam.extras_mut().pattern = Some(Box::new(PatternTag::Mark(Mark::Seam)));
    seam.appearance = seam_look().into();
    let mut children = vec![Arc::new(seam)];
    children.extend(inside.into_iter().map(Arc::new));
    let mut group = Node::group(gid, children);
    group.extras_mut().pattern = Some(Box::new(PatternTag::Piece(Box::new(piece))));
    group
}

/// Draw the parts of the piece group `id` again from its seam line and data, keeping their ids.
pub(crate) fn rebuild(doc: &mut Document, id: NodeId) -> Result<()> {
    let Some(group) = doc.node(id) else { return Ok(()) };
    let Some(mut piece) = piece_on(group).cloned() else { return Ok(()) };
    let children: Vec<Arc<Node>> = group.children().cloned().unwrap_or_default();
    let seam = children.iter().find(|c| mark_of(c) == Some(Mark::Seam)).cloned();
    let old = |m: Mark| children.iter().find(|c| mark_of(c) == Some(m)).map(|c| c.id);
    // Children that are neither the seam line nor a generated part (added by the user): kept, above.
    let others: Vec<Arc<Node>> = children.iter().filter(|c| mark_of(c).is_none()).cloned().collect();
    let mut parts: Vec<Arc<Node>> = Vec::new();
    let mm = mm_to_pt(1.0);
    let geometry = seam.as_ref().and_then(|s| match &s.kind {
        NodeKind::Path { path, .. } => outline_of(path),
        _ => None,
    });
    let ids = |m: Mark, doc: &mut Document| old(m).unwrap_or_else(|| doc.alloc_id());
    if let Some((outline, edges)) = &geometry {
        piece.fit_edges(*edges);
        if let Ok(cut) = pf_core::cutting_line(outline, |e| mm_to_pt(piece.allowance_of(e)), |c| piece.corner_of(c)) {
            let a = Appearance::basic(Paint::None, black(), 1.0);
            parts.push(Arc::new(mark_node(ids(Mark::Cut, doc), Mark::Cut, path_kind(vec![polyline(&cut, true)]), a)));
        }
    }
    if let Some(s) = &seam {
        parts.push(s.clone());
    }
    if let Some((outline, _)) = &geometry {
        let grain = piece.grainline.or_else(|| pf_core::marks::default_grainline(outline));
        if let Some(g) = grain {
            let lines = pf_core::marks::grainline(&g, mm);
            if !lines.is_empty() {
                let a = Appearance::basic(Paint::None, black(), 0.75);
                let sub = lines.iter().map(|l| polyline(l, false)).collect();
                parts.push(Arc::new(mark_node(ids(Mark::Grainline, doc), Mark::Grainline, path_kind(sub), a)));
            }
        }
        let notches: Vec<SubPath> = piece
            .notches
            .iter()
            .flat_map(|n| pf_core::marks::notch(outline, n, mm_to_pt(piece.allowance_of(n.edge)), mm))
            .map(|l| polyline(&l, false))
            .collect();
        if !notches.is_empty() {
            let a = Appearance::basic(Paint::None, black(), 1.0);
            parts.push(Arc::new(mark_node(ids(Mark::Notches, doc), Mark::Notches, path_kind(notches), a)));
        }
        let drills: Vec<SubPath> = piece.drills.iter().flat_map(|d| pf_core::marks::drill(d, mm)).map(|l| polyline(&l, false)).collect();
        if !drills.is_empty() {
            let a = Appearance::basic(Paint::None, black(), 0.75);
            parts.push(Arc::new(mark_node(ids(Mark::Drills, doc), Mark::Drills, path_kind(drills), a)));
        }
        // Designs placed on the piece: each one's outline and crosshair, at its true size.
        let placed: Vec<SubPath> = piece.placements.iter().flat_map(|pl| pl.marks()).map(|l| polyline(&l, false)).collect();
        if !placed.is_empty() {
            let a = Appearance::basic(Paint::None, black(), 0.75);
            parts.push(Arc::new(mark_node(ids(Mark::Placements, doc), Mark::Placements, path_kind(placed), a)));
        }
        // The label, beside the grainline's middle, on the side of the piece with more room.
        let at = grain.map(|g| (g.from + g.to) * 0.5).unwrap_or_default();
        let (left, right) = outline.points.iter().fold((at.x, at.x), |(l, r), p| (l.min(p.x), r.max(p.x)));
        let side = if at.x - left > right - at.x { -1.0 } else { 1.0 };
        let style = CharStyle { size: LABEL_PT, fill: black(), ..Default::default() };
        let mut text = TextObject::point(Point::ZERO, &piece.label(), style);
        text.para.justify = vectorcraft_doc::Justify::Center;
        super::typecmd::refresh_bounds(&mut text);
        let bounds = text.cached_bounds.unwrap_or_else(|| text.estimate_bounds());
        let shift = Point::new(at.x + side * (bounds.width() / 2.0 + 3.0 * mm), at.y);
        text.xf = Affine::translate(shift.to_vec2() - bounds.center().to_vec2());
        let a = Appearance::basic(black(), Paint::None, 0.0);
        parts.push(Arc::new(mark_node(ids(Mark::Label, doc), Mark::Label, NodeKind::Text(Box::new(text)), a)));
    }
    parts.extend(others);
    if let Some(n) = doc.node_mut(id) {
        n.name = Some(if piece.name.is_empty() { "Pattern piece".into() } else { piece.name.clone() });
        n.kind = NodeKind::Group { children: parts, clip: false };
        n.extras_mut().pattern = Some(Box::new(PatternTag::Piece(Box::new(piece))));
    }
    Ok(())
}

/// The seam line path of piece group `id` in `doc`.
fn seam_path(doc: &Document, id: NodeId) -> Option<PathData> {
    let g = doc.node(id)?;
    g.children()?.iter().find(|c| mark_of(c) == Some(Mark::Seam)).and_then(|c| match &c.kind {
        NodeKind::Path { path, .. } => Some(path.clone()),
        _ => None,
    })
}

/// How many anchors the closed outline of `path` has.
fn anchors_of(path: &PathData) -> Option<usize> {
    path.subpaths.iter().find(|sp| sp.closed).map(|sp| sp.anchors.len())
}

/// The edge of closed outline `sp` nearest `q`, and where along it by length (0 to 1), with the
/// distance.
fn nearest_edge(sp: &SubPath, q: Point) -> Option<(usize, f64, f64)> {
    use vectorcraft_geom::kurbo::{ParamCurve, ParamCurveArclen, ParamCurveNearest};
    let mut best: Option<(usize, f64, f64)> = None;
    for i in 0..sp.segment_count() {
        let seg = sp.segment(i);
        let near = seg.nearest(q, 1e-9);
        let d = near.distance_sq.sqrt();
        if best.is_none_or(|b| d < b.2) {
            let total = seg.arclen(1e-9);
            let at = if total > 1e-9 { seg.subsegment(0.0..near.t).arclen(1e-9) / total } else { 0.5 };
            best = Some((i, at.clamp(0.0, 1.0), d));
        }
    }
    best
}

/// The point `at` of the way along edge `e` of `sp` by length.
pub(crate) fn point_along(sp: &SubPath, e: usize, at: f64) -> Option<Point> {
    use vectorcraft_geom::kurbo::{ParamCurve, ParamCurveArclen};
    if e >= sp.segment_count() {
        return None;
    }
    let seg = sp.segment(e);
    let t = seg.inv_arclen(seg.arclen(1e-9) * at.clamp(0.0, 1.0), 1e-9);
    Some(seg.eval(t))
}

/// A piece's edge data carried from its old seam line to its new one when the seam line gained or
/// lost anchors (a design tool, or an anchor added by hand), so they stay where they were rather
/// than on whichever edge now has their number: each notch at the place it was, each new edge's
/// own allowance that of the old edge its middle lies on (both halves of a split edge keep it),
/// each corner's style at the anchor in the same place, the fold on the edge that lies along it.
fn follow_seam(piece: &mut Piece, old: &PathData, new: &PathData) {
    let (Some(o), Some(n)) = (old.subpaths.iter().find(|sp| sp.closed), new.subpaths.iter().find(|sp| sp.closed)) else { return };
    let near = mm_to_pt(0.5);
    let old_edge_of = |q: Point| nearest_edge(o, q).filter(|b| b.2 <= near).map(|b| b.0);
    let mid = |sp: &SubPath, e: usize| point_along(sp, e, 0.5);
    let notches = std::mem::take(&mut piece.notches);
    for nt in notches {
        let Some(q) = point_along(o, nt.edge, nt.at) else { continue };
        if let Some((edge, at, _)) = nearest_edge(n, q) {
            piece.notches.push(Notch { edge, at, kind: nt.kind });
        }
    }
    let edges = std::mem::take(&mut piece.edges);
    for e in 0..n.segment_count() {
        if let Some(mm) = mid(n, e).and_then(old_edge_of).and_then(|oe| edges.get(&oe)) {
            piece.edges.insert(e, *mm);
        }
    }
    let corners = std::mem::take(&mut piece.corners);
    for (c, style) in corners {
        let Some(p) = o.anchors.get(c).map(|a| a.p) else { continue };
        if let Some(i) = n.anchors.iter().position(|a| (a.p - p).hypot() <= near) {
            piece.corners.insert(i, style);
        }
    }
    // A sewn run's ends at the anchors in the same places; a run whose end went is dropped.
    let at = |a: usize| o.anchors.get(a).and_then(|k| n.anchors.iter().position(|b| (b.p - k.p).hypot() <= near));
    let sewn = std::mem::take(&mut piece.sewn);
    piece.sewn = sewn.into_iter().filter_map(|r| Some(pf_core::Sewn { from: at(r.from)?, to: at(r.to)?, label: r.label })).collect();
    piece.fold = piece.fold.and_then(|f| {
        let on: Vec<usize> = (0..n.segment_count()).filter(|&e| mid(n, e).and_then(old_edge_of) == Some(f)).collect();
        match on[..] {
            [one] => Some(one),
            _ => None,
        }
    });
}

/// Called inside every edit: pieces whose seam line or data changed are drawn again, in the same
/// undo step. Returns at once in documents without pieces.
pub(crate) fn refresh(before: &Document, doc: &mut Document) -> Result<()> {
    if !doc.pieces {
        return Ok(());
    }
    let mut pieces = Vec::new();
    doc.walk(|n| {
        if piece_on(n).is_some() {
            pieces.push(n.id);
        }
    });
    doc.pieces = !pieces.is_empty();
    for id in pieces {
        let (Some(now), then) = (doc.node(id), before.node(id)) else { continue };
        if then.is_some_and(|t| t == now) {
            continue;
        }
        // The seam line gained or lost anchors: the edge data follows it.
        if let (Some(old), Some(new)) = (seam_path(before, id), seam_path(doc, id))
            && anchors_of(&old) != anchors_of(&new)
            && let Some(was) = doc.node(id).and_then(piece_on).cloned()
        {
            // From the edge data as it was before this edit; notches the edit added itself (a
            // pleat's) are already on the new seam line.
            let src = before.node(id).and_then(piece_on).cloned().unwrap_or_else(|| was.clone());
            let added: Vec<Notch> = was.notches.iter().filter(|n| !src.notches.contains(n)).copied().collect();
            let mut piece = was.clone();
            (piece.notches, piece.edges, piece.corners, piece.fold, piece.sewn) = (src.notches, src.edges, src.corners, src.fold, src.sewn);
            follow_seam(&mut piece, &old, &new);
            piece.notches.extend(added);
            piece.notches.sort_by(|a, b| a.edge.cmp(&b.edge).then(a.at.total_cmp(&b.at)));
            if piece != was
                && let Some(n) = doc.node_mut(id)
            {
                n.extras_mut().pattern = Some(Box::new(PatternTag::Piece(Box::new(piece))));
            }
        }
        rebuild(doc, id)?;
    }
    Ok(())
}

fn ids_json(ids: &[NodeId]) -> Value {
    json!({ "ids": ids.iter().map(|i| i.0).collect::<Vec<_>>() })
}

fn make(s: &mut Session, p: &Value) -> Result<Value> {
    const CMD: &str = "piece.make";
    let allowance = match p.get("allowance") {
        Some(v) => allowance_param(v, CMD)?,
        None => pf_core::piece::DEFAULT_ALLOWANCE_MM,
    };
    let name = match str_param(p, "name") {
        Some(n) => pf_core::piece::check_name(n).map_err(|e| err(CMD, e))?,
        None => String::new(),
    };
    // Given ids are taken as the same objects selected would be (an object inside a listed group
    // stands for nothing more than the group).
    let roots = if p.get("ids").is_some() {
        let ids = checked_ids_param(s, p, "ids", CMD)?;
        let st = s.doc()?;
        super::edit::roots_of(&st.doc, ids)
    } else {
        super::edit::selected_roots(s)?
    };
    if roots.len() > MAX_PIECES {
        return Err(bad(CMD, "too many objects at once"));
    }
    let ids = s.edit("Make Pattern Piece", |d, sel| {
        let mut made = Vec::new();
        let mut beside = Vec::new();
        for id in roots {
            let Some(n) = d.node(id) else { continue };
            if piece_group(d, id).is_some() {
                continue;
            }
            let NodeKind::Path { path, .. } = &n.kind else { continue };
            if !path.subpaths.iter().any(|sp| sp.closed) {
                continue;
            }
            let mut seam = n.clone();
            seam.extras_mut().pattern = Some(Box::new(PatternTag::Mark(Mark::Seam)));
            // Patterns draw the seam (stitching) line dashed and the cutting line solid.
            seam.appearance = seam_look().into();
            let gid = d.alloc_id();
            let mut group = Node::group(gid, vec![Arc::new(seam)]);
            let piece = Piece { name: name.clone(), allowance, ..Piece::default() };
            group.extras_mut().pattern = Some(Box::new(PatternTag::Piece(Box::new(piece))));
            beside.push((id, vec![group]));
            made.push(gid);
        }
        if made.is_empty() {
            return Err(bad(CMD, "select closed paths: each becomes a piece, the path its seam line"));
        }
        d.insert_beside(beside, true)?;
        d.pieces = true;
        for &g in &made {
            rebuild(d, g)?;
        }
        sel.set(made.iter().copied());
        Ok(made)
    })?;
    Ok(ids_json(&ids))
}

fn release(s: &mut Session, p: &Value) -> Result<Value> {
    const CMD: &str = "piece.release";
    let pieces = target_pieces(s, p, CMD)?;
    let ids = s.edit("Release Pattern Piece", |d, sel| {
        let mut out = Vec::new();
        let mut beside = Vec::new();
        for g in pieces {
            let Some(group) = d.node(g) else { continue };
            let children = group.children().cloned().unwrap_or_default();
            let mut keep: Vec<Node> = Vec::new();
            for c in children {
                match mark_of(&c) {
                    Some(Mark::Seam) => {
                        let mut seam = (*c).clone();
                        seam.extras_mut().pattern = None;
                        out.push(seam.id);
                        keep.push(seam);
                    }
                    Some(_) => {}
                    None => keep.push((*c).clone()),
                }
            }
            beside.push((g, keep));
        }
        d.insert_beside(beside, true)?;
        sel.set(out.iter().copied());
        Ok(out)
    })?;
    Ok(ids_json(&ids))
}

/// Change the piece `id` with `f` (checked first on a copy), in one undo step labelled `label`.
fn change(s: &mut Session, id: NodeId, label: &str, cmd: &str, f: impl FnOnce(&mut Piece) -> Result<()>) -> Result<Value> {
    let mut piece = s.doc()?.doc.node(id).and_then(piece_on).cloned().ok_or_else(|| bad(cmd, "not a pattern piece"))?;
    f(&mut piece)?;
    s.edit(label, |d, _| {
        if let Some(n) = d.node_mut(id) {
            n.extras_mut().pattern = Some(Box::new(PatternTag::Piece(Box::new(piece))));
        }
        rebuild(d, id)
    })?;
    info_of(&s.doc()?.doc, id)
}

fn key_index(k: &str, cmd: &str) -> Result<usize> {
    k.trim().parse::<usize>().ok().filter(|i| *i < 100_000).ok_or_else(|| bad(cmd, "edges and corners are numbered from 0"))
}

fn point_of(v: &Value) -> Option<Pt> {
    let [x, y] = finite_numbers::<2>(v)?;
    Some(Pt::new(x, y))
}

fn set(s: &mut Session, p: &Value) -> Result<Value> {
    const CMD: &str = "piece.set";
    let id = one_piece(s, p, CMD)?;
    // The fold: a straight edge of the seam line.
    let fold = match p.get("fold").or(p.get("foldAt")) {
        None => None,
        Some(v) if p.get("foldAt").is_some() => {
            let q = point_of(v).ok_or_else(|| bad(CMD, "foldAt: [x, y] in document points"))?;
            let (e, _) = edge_near(s, id, Point::new(q.x, q.y)).ok_or_else(|| bad(CMD, "the point is not on the seam line or in its allowance"))?;
            if !straight(s, id, e) {
                return Err(bad(CMD, "the fold must be a straight edge"));
            }
            Some(Some(e))
        }
        Some(Value::Null) => Some(None),
        Some(Value::String(a)) if a == "auto" => {
            Some(Some(longest_straight(s, id).ok_or_else(|| bad(CMD, "the piece has no straight edge to cut on the fold"))?))
        }
        Some(v) => {
            let e = v.as_u64().map(|e| e as usize).ok_or_else(|| bad(CMD, "fold is an edge's number (0..), or null"))?;
            if e >= edge_count(s, id) {
                return Err(bad(CMD, format!("the piece has no edge {e}")));
            }
            if !straight(s, id, e) {
                return Err(bad(CMD, "the fold must be a straight edge"));
            }
            Some(Some(e))
        }
    };
    // One edge's allowance, the edge picked at a point (as a click on the canvas).
    let edge_allowance = match p.get("edgeAllowance") {
        None => None,
        Some(v) => {
            let q = p.get("at").and_then(point_of).ok_or_else(|| bad(CMD, "at: [x, y], the document point on the edge"))?;
            let (e, _) = edge_near(s, id, Point::new(q.x, q.y)).ok_or_else(|| bad(CMD, "the point is not on the seam line or in its allowance"))?;
            Some((e, if v.is_null() { None } else { Some(allowance_param(v, CMD)?) }))
        }
    };
    change(s, id, "Piece Options", CMD, |piece| {
        if let Some(f) = fold {
            piece.fold = f;
        }
        match edge_allowance {
            Some((e, Some(mm))) => {
                piece.edges.insert(e, mm);
            }
            Some((e, None)) => {
                piece.edges.remove(&e);
            }
            None => {}
        }
        if let Some(n) = str_param(p, "name") {
            piece.name = pf_core::piece::check_name(n).map_err(|e| err(CMD, e))?;
        }
        if let Some(f) = str_param(p, "fabric") {
            piece.fabric = pf_core::piece::check_name(f).map_err(|e| err(CMD, e))?;
        }
        if let Some(v) = p.get("cut") {
            let cut = v.as_u64().and_then(|c| u32::try_from(c).ok()).ok_or_else(|| bad(CMD, "cut is a whole number, 1 to 999"))?;
            piece.set_cut(cut).map_err(|e| err(CMD, e))?;
        }
        if let Some(v) = p.get("mirror") {
            piece.mirror = v.as_bool().ok_or_else(|| bad(CMD, "mirror is true or false"))?;
        }
        if let Some(v) = p.get("allowance") {
            piece.allowance = allowance_param(v, CMD)?;
        }
        if let Some(edges) = p.get("edges") {
            let map = edges.as_object().ok_or_else(|| bad(CMD, "edges is {\"<edge>\": mm or null}"))?;
            for (k, v) in map {
                let e = key_index(k, CMD)?;
                if v.is_null() {
                    piece.edges.remove(&e);
                } else {
                    piece.edges.insert(e, allowance_param(v, CMD)?);
                }
            }
        }
        if let Some(corners) = p.get("corners") {
            let map = corners.as_object().ok_or_else(|| bad(CMD, "corners is {\"<corner>\": \"mitre\" or \"square\"}"))?;
            for (k, v) in map {
                let c = key_index(k, CMD)?;
                match v.as_str() {
                    Some("mitre" | "miter") => {
                        piece.corners.remove(&c);
                    }
                    Some("square") => {
                        piece.corners.insert(c, Corner::Square);
                    }
                    _ => return Err(bad(CMD, "a corner is \"mitre\" or \"square\"")),
                }
            }
        }
        match p.get("grainline") {
            None => {}
            Some(Value::String(a)) if a == "auto" => piece.grainline = None,
            Some(Value::Null) => piece.grainline = None,
            Some(v) => {
                let (Some(from), Some(to)) = (v.get("from").and_then(point_of), v.get("to").and_then(point_of)) else {
                    return Err(bad(CMD, "grainline is {from: [x, y], to: [x, y]} or \"auto\""));
                };
                if (to - from).len() < 1e-6 {
                    return Err(bad(CMD, "the grainline's ends are the same point"));
                }
                piece.grainline = Some(pf_core::Grainline { from, to });
            }
        }
        Ok(())
    })
}

/// How far past an edge's seam allowance a point picked on the canvas may be from the seam line, in
/// mm, and still name that edge.
const PICK_MM: f64 = 5.0;

/// The edge of piece `id`'s seam line nearest document point `q`, and how far along it (0 to 1,
/// by length), when `q` is within the edge's allowance and [`PICK_MM`] of the seam line.
fn edge_near(s: &Session, id: NodeId, q: Point) -> Option<(usize, f64)> {
    use vectorcraft_geom::kurbo::{ParamCurve, ParamCurveArclen, ParamCurveNearest};
    let st = s.doc().ok()?;
    let piece = st.doc.node(id).and_then(piece_on)?;
    let seam = seam_of(&st.doc, id)?;
    let NodeKind::Path { path, .. } = &seam.kind else { return None };
    let sp = path.subpaths.iter().find(|sp| sp.closed)?;
    let (mut best, mut best_d) = (None, f64::INFINITY);
    for i in 0..sp.segment_count() {
        let seg = sp.segment(i);
        let near = seg.nearest(q, 1e-6);
        if near.distance_sq < best_d {
            best_d = near.distance_sq;
            let total = seg.arclen(1e-6);
            let at = if total > 1e-9 { seg.subsegment(0.0..near.t).arclen(1e-6) / total } else { 0.5 };
            best = Some((i, at.clamp(0.0, 1.0)));
        }
    }
    let (edge, at) = best?;
    (best_d.sqrt() <= mm_to_pt(piece.allowance_of(edge) + PICK_MM)).then_some((edge, at))
}

fn add_notch(s: &mut Session, p: &Value) -> Result<Value> {
    const CMD: &str = "piece.addNotch";
    let id = one_piece(s, p, CMD)?;
    let (edge, at) = match p.get("point") {
        Some(v) => {
            let q = point_of(v).ok_or_else(|| bad(CMD, "point: [x, y] in document points"))?;
            edge_near(s, id, Point::new(q.x, q.y)).ok_or_else(|| bad(CMD, "the point is not on the seam line or in its allowance"))?
        }
        None => {
            let edge = p
                .get("edge")
                .and_then(Value::as_u64)
                .and_then(|e| usize::try_from(e).ok())
                .ok_or_else(|| bad(CMD, "edge: the edge's number, from 0 (or point)"))?;
            let at = match p.get("at") {
                None => 0.5,
                Some(v) => v.as_f64().ok_or_else(|| bad(CMD, "at: 0 (the edge's start) to 1 (its end)"))?,
            };
            (edge, at)
        }
    };
    let kind = notch_kind(p, CMD)?;
    let edges = edge_count(s, id);
    if edge >= edges {
        return Err(bad(CMD, format!("the piece has edges 0 to {}", edges.saturating_sub(1))));
    }
    change(s, id, "Add Notch", CMD, |piece| piece.add_notch(Notch { edge, at, kind }).map_err(|e| err(CMD, e)))
}

/// The `kind` param: a slit when there is none.
fn notch_kind(p: &Value, cmd: &str) -> Result<NotchKind> {
    match str_param(p, "kind") {
        None | Some("slit") => Ok(NotchKind::Slit),
        Some("double") => Ok(NotchKind::Double),
        Some("v") => Ok(NotchKind::V),
        Some("t") => Ok(NotchKind::T),
        Some("castle") => Ok(NotchKind::Castle),
        Some("u") => Ok(NotchKind::U),
        Some(_) => Err(bad(cmd, "kind: \"slit\", \"double\", \"v\", \"t\", \"castle\" or \"u\"")),
    }
}

fn set_notch(s: &mut Session, p: &Value) -> Result<Value> {
    const CMD: &str = "piece.setNotch";
    let id = one_piece(s, p, CMD)?;
    let index = p
        .get("index")
        .and_then(Value::as_u64)
        .and_then(|i| usize::try_from(i).ok())
        .ok_or_else(|| bad(CMD, "index: the notch's place in the list, from 0"))?;
    if p.get("kind").is_none() {
        return Err(bad(CMD, "kind: the notch's new shape"));
    }
    let kind = notch_kind(p, CMD)?;
    change(s, id, "Notch Shape", CMD, |piece| {
        let n = piece.notches.get_mut(index).ok_or_else(|| bad(CMD, "no notch with that index"))?;
        n.kind = kind;
        Ok(())
    })
}

fn remove_notch(s: &mut Session, p: &Value) -> Result<Value> {
    const CMD: &str = "piece.removeNotch";
    let id = one_piece(s, p, CMD)?;
    let index = p
        .get("index")
        .and_then(Value::as_u64)
        .and_then(|i| usize::try_from(i).ok())
        .ok_or_else(|| bad(CMD, "index: the notch's place in the list, from 0"))?;
    change(s, id, "Remove Notch", CMD, |piece| {
        if index >= piece.notches.len() {
            return Err(bad(CMD, "no notch with that index"));
        }
        piece.notches.remove(index);
        Ok(())
    })
}

fn add_drill(s: &mut Session, p: &Value) -> Result<Value> {
    const CMD: &str = "piece.addDrill";
    let id = one_piece(s, p, CMD)?;
    let at = p.get("point").and_then(point_of).ok_or_else(|| bad(CMD, "point: [x, y] in document points"))?;
    let diameter = match p.get("diameter") {
        None | Some(Value::Null) => 0.0,
        Some(v) => mm_param(v).ok_or_else(|| bad(CMD, "diameter: a number of mm, or a length with its unit (\"1/8 in\")"))?,
    };
    if !inside_cut(s, id, at) {
        return Err(bad(CMD, "the point is not inside the piece"));
    }
    change(s, id, "Add Drill Hole", CMD, |piece| piece.add_drill(pf_core::Drill { at, diameter }).map_err(|e| err(CMD, e)))
}

/// A run param `{id, from, to}` of `piece.sew`: the piece and its anchors, checked against its
/// seam line.
fn run_param(s: &Session, p: &Value, key: &str, cmd: &str) -> Result<(NodeId, usize, usize)> {
    let v = p.get(key).filter(|v| v.is_object()).ok_or_else(|| bad(cmd, format!("{key}: {{id, from: anchor, to: anchor}}")))?;
    let id = one_piece(s, &json!({"id": v.get("id").cloned().unwrap_or(Value::Null)}), cmd)?;
    let anchor = |k: &str| v.get(k).and_then(Value::as_u64).and_then(|a| usize::try_from(a).ok());
    let (Some(from), Some(to)) = (anchor("from"), anchor("to")) else {
        return Err(bad(cmd, format!("{key}: from and to, anchors of the seam line (from 0)")));
    };
    let n = seam_of(&s.doc()?.doc, id)
        .and_then(|seam| match &seam.kind {
            NodeKind::Path { path, .. } => path.subpaths.iter().find(|sp| sp.closed).map(|sp| sp.anchors.len()),
            _ => None,
        })
        .unwrap_or(0);
    if from >= n || to >= n || from == to {
        return Err(bad(cmd, format!("{key}: two different anchors of the seam line, 0 to {}", n.saturating_sub(1))));
    }
    Ok((id, from, to))
}

/// The length along piece `id`'s seam line from anchor `from` forward to anchor `to`, in mm.
fn run_mm(doc: &Document, id: NodeId, from: usize, to: usize) -> f64 {
    use vectorcraft_geom::kurbo::ParamCurveArclen;
    let Some(seam) = seam_of(doc, id) else { return 0.0 };
    let NodeKind::Path { path, .. } = &seam.kind else { return 0.0 };
    let Some(sp) = path.subpaths.iter().find(|sp| sp.closed) else { return 0.0 };
    let n = sp.segment_count();
    if n == 0 {
        return 0.0;
    }
    let mut e = from % n;
    let mut total = 0.0;
    for _ in 0..n {
        total += sp.segment(e).arclen(1e-6);
        e = (e + 1) % n;
        if e == to % n {
            break;
        }
    }
    pt_to_mm(total)
}

/// Every sewn run in `doc`: (label, piece, its name, from, to).
pub(crate) fn sewn_runs(doc: &Document) -> Vec<(String, NodeId, String, usize, usize)> {
    let mut out = Vec::new();
    doc.walk(|n| {
        if let Some(pc) = piece_on(n) {
            for r in &pc.sewn {
                out.push((r.label.clone(), n.id, pc.name.clone(), r.from, r.to));
            }
        }
    });
    out
}

fn sew(s: &mut Session, p: &Value) -> Result<Value> {
    const CMD: &str = "piece.sew";
    let (a, af, at) = run_param(s, p, "a", CMD)?;
    let (b, bf, bt) = run_param(s, p, "b", CMD)?;
    let doc = &s.doc()?.doc;
    // A run sewn to itself is a centre seam: the same edge of a mirror-image pair's two copies
    // (trousers' centre front and back).
    if a == b && (af, at) == (bf, bt) && !doc.node(a).and_then(piece_on).is_some_and(|pc| pc.cut >= 2 && pc.mirror) {
        return Err(bad(
            CMD,
            "a and b are the same run: a seam is sewn to another, or to the same run of a piece cut as a mirror-image pair (a centre seam)",
        ));
    }
    let used: std::collections::BTreeSet<String> = sewn_runs(doc).into_iter().map(|r| r.0).collect();
    let label = match str_param(p, "label").map(str::trim) {
        Some(l) if !l.is_empty() => {
            if used.contains(l) {
                return Err(bad(CMD, "label: already used by other seams (piece.unsew it first)"));
            }
            if l.chars().count() > 20 {
                return Err(bad(CMD, "label: at most 20 characters"));
            }
            l.to_string()
        }
        _ => {
            // A, B… Z, then A2, B2…
            let letter = |i: usize| {
                let c = char::from(b'A' + (i % 26) as u8);
                if i < 26 { c.to_string() } else { format!("{c}{}", i / 26 + 1) }
            };
            (0..).map(letter).find(|l| !used.contains(l)).unwrap_or_default()
        }
    };
    let (am, bm) = (run_mm(doc, a, af, at), run_mm(doc, b, bf, bt));
    let runs = [(a, af, at), (b, bf, bt)];
    s.edit("Sew Seams", |d, _| {
        for (id, from, to) in runs {
            let Some(mut piece) = d.node(id).and_then(piece_on).cloned() else { continue };
            piece.sewn.push(pf_core::Sewn { label: label.clone(), from, to });
            if let Some(n) = d.node_mut(id) {
                n.extras_mut().pattern = Some(Box::new(PatternTag::Piece(Box::new(piece))));
            }
        }
        Ok(())
    })?;
    let r2 = |v: f64| (v * 100.0).round() / 100.0;
    Ok(json!({"label": label, "a": r2(am), "b": r2(bm), "difference": r2(am - bm)}))
}

fn unsew(s: &mut Session, p: &Value) -> Result<Value> {
    const CMD: &str = "piece.unsew";
    let label =
        str_param(p, "label").map(str::trim).filter(|l| !l.is_empty()).ok_or_else(|| bad(CMD, "label: the seams' label (piece.seams)"))?.to_string();
    let ids: Vec<NodeId> = sewn_runs(&s.doc()?.doc).into_iter().filter(|r| r.0 == label).map(|r| r.1).collect();
    if ids.is_empty() {
        return Err(bad(CMD, "no seams are sewn under that label"));
    }
    let count = s.edit("Unsew Seams", |d, _| {
        let mut count = 0;
        for id in &ids {
            let Some(mut piece) = d.node(*id).and_then(piece_on).cloned() else { continue };
            let before = piece.sewn.len();
            piece.sewn.retain(|r| r.label != label);
            count += before - piece.sewn.len();
            if let Some(n) = d.node_mut(*id) {
                n.extras_mut().pattern = Some(Box::new(PatternTag::Piece(Box::new(piece))));
            }
        }
        Ok(count)
    })?;
    Ok(json!({"label": label, "runs": count}))
}

fn seams(s: &mut Session, _: &Value) -> Result<Value> {
    let doc = &s.doc()?.doc;
    let mut by: std::collections::BTreeMap<String, Vec<Value>> = std::collections::BTreeMap::new();
    for (label, id, name, from, to) in sewn_runs(doc) {
        let mm = (run_mm(doc, id, from, to) * 100.0).round() / 100.0;
        by.entry(label).or_default().push(json!({"piece": id.0, "name": name, "from": from, "to": to, "mm": mm}));
    }
    let list: Vec<Value> = by
        .into_iter()
        .map(|(label, runs)| {
            let mm = |i: usize| runs.get(i).and_then(|r| r["mm"].as_f64());
            let difference = match (mm(0), mm(1)) {
                (Some(a), Some(b)) => json!(((a - b) * 100.0).round() / 100.0),
                _ => Value::Null,
            };
            json!({"label": label, "runs": runs, "difference": difference})
        })
        .collect();
    Ok(json!({"seams": list}))
}

fn remove_drill(s: &mut Session, p: &Value) -> Result<Value> {
    const CMD: &str = "piece.removeDrill";
    let id = one_piece(s, p, CMD)?;
    let drills = s.doc()?.doc.node(id).and_then(piece_on).map(|pc| pc.drills.clone()).unwrap_or_default();
    let index = match p.get("point") {
        Some(v) => {
            let q = point_of(v).ok_or_else(|| bad(CMD, "point: [x, y] in document points"))?;
            drills
                .iter()
                .enumerate()
                .map(|(i, d)| (i, (d.at - q).len()))
                .filter(|(_, dist)| *dist <= mm_to_pt(PICK_MM))
                .min_by(|a, b| a.1.total_cmp(&b.1))
                .map(|(i, _)| i)
                .ok_or_else(|| bad(CMD, "no drill hole near the point"))?
        }
        None => p
            .get("index")
            .and_then(Value::as_u64)
            .and_then(|i| usize::try_from(i).ok())
            .ok_or_else(|| bad(CMD, "index: the drill hole's place in the list, from 0 (or point)"))?,
    };
    change(s, id, "Remove Drill Hole", CMD, |piece| {
        if index >= piece.drills.len() {
            return Err(bad(CMD, "no drill hole with that index"));
        }
        piece.drills.remove(index);
        Ok(())
    })
}

/// Whether document point `q` is inside piece `id`'s cutting line (its seam line when it has none).
fn inside_cut(s: &Session, id: NodeId, q: Pt) -> bool {
    let Ok(st) = s.doc() else { return false };
    let Some(piece) = st.doc.node(id).and_then(piece_on) else { return false };
    let Some(seam) = seam_of(&st.doc, id) else { return false };
    let NodeKind::Path { path, .. } = &seam.kind else { return false };
    let Some((outline, _)) = outline_of(path) else { return false };
    let ring = pf_core::cutting_line(&outline, |e| mm_to_pt(piece.allowance_of(e)), |c| piece.corner_of(c)).unwrap_or(outline.points);
    // Even-odd: a ray to the right crosses the ring an odd number of times from inside.
    let mut inside = false;
    for i in 0..ring.len() {
        let (a, b) = (ring[i], ring[(i + 1) % ring.len()]);
        if (a.y > q.y) != (b.y > q.y) && q.x < a.x + (q.y - a.y) / (b.y - a.y) * (b.x - a.x) {
            inside = !inside;
        }
    }
    inside
}

pub(crate) fn seam_of(doc: &Document, id: NodeId) -> Option<Arc<Node>> {
    doc.node(id)?.children()?.iter().find(|c| mark_of(c) == Some(Mark::Seam)).cloned()
}

fn edge_count(s: &Session, id: NodeId) -> usize {
    let Ok(st) = s.doc() else { return 0 };
    seam_of(&st.doc, id)
        .and_then(|n| match &n.kind {
            NodeKind::Path { path, .. } => outline_of(path).map(|(_, e)| e),
            _ => None,
        })
        .unwrap_or(0)
}

/// The longest straight edge of piece `id`'s seam line.
fn longest_straight(s: &Session, id: NodeId) -> Option<usize> {
    let st = s.doc().ok()?;
    let seam = seam_of(&st.doc, id)?;
    let NodeKind::Path { path, .. } = &seam.kind else { return None };
    let sp = path.subpaths.iter().find(|sp| sp.closed)?;
    (0..sp.segment_count()).filter(|&e| sp.segment_is_line(e)).max_by(|&a, &b| {
        let len = |e: usize| {
            let g = sp.segment(e);
            (g.p3 - g.p0).hypot()
        };
        len(a).total_cmp(&len(b))
    })
}

/// Whether edge `e` of piece `id`'s seam line is straight.
fn straight(s: &Session, id: NodeId, e: usize) -> bool {
    let Ok(st) = s.doc() else { return false };
    seam_of(&st.doc, id).is_some_and(|n| match &n.kind {
        NodeKind::Path { path, .. } => path.subpaths.iter().find(|sp| sp.closed).is_some_and(|sp| e < sp.segment_count() && sp.segment_is_line(e)),
        _ => false,
    })
}

fn info_of(doc: &Document, id: NodeId) -> Result<Value> {
    let n = doc.node(id).ok_or_else(|| bad("piece.info", "no object with that id"))?;
    let piece = piece_on(n).ok_or_else(|| bad("piece.info", "not a pattern piece"))?;
    let seam = seam_of(doc, id);
    let mut v = serde_json::to_value(piece).map_err(|e| err("piece.info", e))?;
    let (mut edges, mut length, mut area) = (0, 0.0, 0.0);
    if let Some(NodeKind::Path { path, .. }) = seam.as_deref().map(|s| &s.kind)
        && let Some(sp) = path.subpaths.iter().find(|sp| sp.closed)
    {
        edges = sp.segment_count();
        length = pt_to_mm(PathData::single(sp.clone()).length());
        area = pt_to_mm(pt_to_mm(sp.area().abs())) / 100.0;
    }
    if let Some(o) = v.as_object_mut() {
        o.insert("id".into(), json!(id.0));
        o.insert("seam".into(), json!(seam.as_ref().map(|s| s.id.0)));
        o.insert("edgeCount".into(), json!(edges));
        o.insert("seamLength".into(), json!((length * 100.0).round() / 100.0));
        o.insert("area".into(), json!((area * 100.0).round() / 100.0));
        o.insert("problem".into(), json!(problem(seam.as_deref())));
    }
    Ok(v)
}

fn info(s: &mut Session, p: &Value) -> Result<Value> {
    let id = one_piece(s, p, "piece.info")?;
    info_of(&s.doc()?.doc, id)
}

fn list(s: &mut Session, _: &Value) -> Result<Value> {
    let doc = &s.doc()?.doc;
    let mut pieces = Vec::new();
    doc.walk(|n| {
        if let Some(pc) = piece_on(n) {
            pieces.push(json!({"id": n.id.0, "name": pc.name, "cut": pc.cut, "mirror": pc.mirror, "fabric": pc.fabric}));
        }
    });
    Ok(json!({ "pieces": pieces }))
}

fn outlines(s: &mut Session, p: &Value) -> Result<Value> {
    const CMD: &str = "piece.outlines";
    let open = match p.get("openFolds") {
        None | Some(Value::Null) => true,
        Some(v) => v.as_bool().ok_or_else(|| bad(CMD, "openFolds: true or false"))?,
    };
    let doc = &s.doc()?.doc;
    let sized = match p.get("size") {
        None | Some(Value::Null) => None,
        Some(Value::String(size)) if size.trim().is_empty() => None,
        Some(Value::String(size)) => Some(super::grade::in_size(doc, size, CMD)?),
        Some(_) => return Err(bad(CMD, "size: the name of a size of the size range or chart")),
    };
    let pieces = super::fileio::aama::pieces(sized.as_ref().unwrap_or(doc))?;
    let mm = |q: [f64; 2]| [(pt_to_mm(q[0]) * 1000.0).round() / 1000.0, (pt_to_mm(q[1]) * 1000.0).round() / 1000.0];
    let ring = |v: &[[f64; 2]]| v.iter().map(|q| mm(*q)).collect::<Vec<_>>();
    let (mut x0, mut y0, mut x1, mut y1) = (f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);
    let mut out = Vec::new();
    for piece in &pieces {
        let piece = if open { super::fileio::aama::opened(piece) } else { piece.clone() };
        for q in piece.boundary.iter().map(|q| mm(*q)) {
            (x0, y0, x1, y1) = (x0.min(q[0]), y0.min(q[1]), x1.max(q[0]), y1.max(q[1]));
        }
        // Each notch as the cut it makes: its angle is counter-clockwise with y up, y runs down.
        let notches: Vec<Vec<[f64; 2]>> = piece.notches.iter().flat_map(|n| n.lines(true)).map(|l| ring(&l)).collect();
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
        for note in &piece.notes {
            text.push('\n');
            text.push_str(note);
        }
        out.push(json!({
            "name": piece.name,
            "text": text,
            "cut": ring(&piece.boundary),
            "seam": ring(&piece.sew_line),
            "notches": notches,
            "grain": piece.grain.map(|g| [mm(g[0]), mm(g[1])]),
            "fold": piece.mirror.map(|g| [mm(g[0]), mm(g[1])]),
            "lines": piece.internal.iter().map(|l| ring(l)).collect::<Vec<_>>(),
            "drills": piece.drills.iter().map(|d| json!({"at": mm(d.at), "diameter": (pt_to_mm(d.diameter) * 1000.0).round() / 1000.0})).collect::<Vec<_>>(),
            "label": mm(piece.text_at),
        }));
    }
    let bounds = if out.is_empty() { Value::Null } else { json!([x0, y0, x1, y1]) };
    Ok(json!({"pieces": out, "bounds": bounds}))
}
