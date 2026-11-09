//! DXF-AAMA export (`document.export {format: aama}`, PF-7.1): the document's pattern pieces
//! as an ASTM D6673 file for cutting rooms, grading and nesting systems
//! ([`pf_aama::write`]). Each piece gives its cutting line (the boundary), its seam
//! line (the sew line), its notches (on the cutting line, cut toward the seam line; slits and V
//! notches on layer 4, T, castle and U notches on 80, 81 and 83, PF-7.7), its grainline,
//! the fold it is cut on as its mirror line (PF-7.4, the piece written as the half), its drill
//! holes (PF-7.6, with the punch's diameter), and the
//! paths the user added inside it as internal lines; in millimetres, y up, every piece
//! where it is in the document relative to the others.

use pf_aama::{Drill as AamaDrill, Mm, Notch as AamaNotch, NotchShape, Piece as AamaPiece, Style as AamaStyle, Version};
use serde_json::Value;
use vectorcraft_doc::{Document, Node, NodeKind};
use vectorcraft_geom::{BezPath, SubPath};

use super::super::piece::{FLATTEN_PT, mark_of, outline_of, piece_on};
use super::super::*;
use super::{Encoded, FormatOption};
use pf_core::{Mark, NotchKind, Pt, mm_to_pt, pt_to_mm};

const C: &str = "document.export";
/// Room under the pieces for the style system text, which sits at the origin (mm).
const TEXT_ROOM_MM: f64 = 60.0;

/// The DXF-AAMA options `document.formats` lists.
pub(super) const OPTIONS: &[FormatOption] = &[
    FormatOption { name: "version", ty: "string", default: "\"R13\"", description: "R13 (ASTM D6673) or R12 (for older AAMA readers)" },
    FormatOption { name: "styleName", ty: "string", default: "\"Pattern\"", description: "the Style Name the file gives the pattern" },
    FormatOption {
        name: "size",
        ty: "string",
        default: "\"\"",
        description: "write the pieces graded to this size of the size chart or size range (default: as drawn)",
    },
    FormatOption { name: "sampleSize", ty: "string", default: "\"\"", description: "the Sample Size, the size the pieces are drafted in" },
];

/// A subpath as points about [`FLATTEN_PT`] from its curves.
fn flatten(sp: &SubPath) -> Vec<Pt> {
    let mut out = Vec::new();
    for i in 0..sp.segment_count() {
        let seg = sp.segment(i);
        if out.is_empty() {
            out.push(Pt::new(seg.p0.x, seg.p0.y));
        }
        if sp.segment_is_line(i) {
            out.push(Pt::new(seg.p3.x, seg.p3.y));
            continue;
        }
        let mut bez = BezPath::new();
        bez.move_to(seg.p0);
        bez.curve_to(seg.p1, seg.p2, seg.p3);
        vectorcraft_geom::kurbo::flatten(bez, FLATTEN_PT, |el| {
            if let vectorcraft_geom::kurbo::PathEl::LineTo(p) = el {
                out.push(Pt::new(p.x, p.y));
            }
        });
    }
    // A closed subpath comes back to its start: the start is not written twice.
    if sp.closed && out.len() > 1 && out.first() == out.last() {
        out.pop();
    }
    out
}

/// The paths drawn inside a piece by the user (not its generated parts), flattened.
fn internal_lines(n: &Node, out: &mut Vec<Vec<Pt>>) {
    match &n.kind {
        NodeKind::Path { path, .. } => {
            for sp in &path.subpaths {
                let mut pts = flatten(sp);
                if sp.closed
                    && let Some(first) = pts.first().copied()
                {
                    pts.push(first);
                }
                if pts.len() >= 2 {
                    out.push(pts);
                }
            }
        }
        NodeKind::Group { children, .. } => children.iter().for_each(|c| internal_lines(c, out)),
        _ => {}
    }
}

/// The pieces of `doc` in paint order, in points (y down).
pub(crate) fn pieces(doc: &Document) -> Result<Vec<AamaPiece>> {
    let mut nodes = Vec::new();
    doc.walk(|n| {
        if piece_on(n).is_some() {
            nodes.push(n.clone());
        }
    });
    let mut out = Vec::new();
    for n in &nodes {
        let Some(piece) = piece_on(n) else { continue };
        let children = n.children().cloned().unwrap_or_default();
        let seam = children.iter().find(|c| mark_of(c) == Some(Mark::Seam));
        // An unnamed piece goes by its seam line's name (a block part's), then "Piece".
        let name = [Some(piece.name.as_str()), seam.and_then(|s| s.name.as_deref())]
            .into_iter()
            .flatten()
            .map(str::trim)
            .find(|n| !n.is_empty())
            .unwrap_or("Piece")
            .to_string();
        let Some(seam) = seam else {
            return Err(bad(C, format!("the piece {name:?} has no seam line")));
        };
        let NodeKind::Path { path, .. } = &seam.kind else {
            return Err(bad(C, format!("the seam line of {name:?} is not a path")));
        };
        let Some((outline, _)) = outline_of(path) else {
            return Err(bad(C, format!("the seam line of {name:?} is not a closed path")));
        };
        let cut = pf_core::cutting_line(&outline, |e| mm_to_pt(piece.allowance_of(e)), |c| piece.corner_of(c))
            .map_err(|e| bad(C, format!("{name}: {e}")))?;
        let mut notches = Vec::new();
        for nt in &piece.notches {
            let Some((p, dir, outward)) = outline.point_on_edge(nt.edge, nt.at) else { continue };
            let allowance = mm_to_pt(piece.allowance_of(nt.edge));
            let at = p + outward * allowance;
            let depth =
                if allowance > 0.0 { mm_to_pt(pf_core::marks::NOTCH_DEPTH_MM).min(allowance) } else { mm_to_pt(pf_core::marks::NOTCH_DEPTH_MM) };
            let inward = Pt::new(-outward.x, -outward.y);
            let mut add = |at: Pt, width: f64, shape: NotchShape| notches.push((at, inward, depth, width, shape));
            let width = mm_to_pt(pf_core::marks::NOTCH_WIDTH_MM);
            match nt.kind {
                NotchKind::Slit => add(at, 0.0, NotchShape::Plain),
                NotchKind::Double => {
                    let h = mm_to_pt(pf_core::marks::DOUBLE_NOTCH_GAP_MM) / 2.0;
                    add(at - dir * h, 0.0, NotchShape::Plain);
                    add(at + dir * h, 0.0, NotchShape::Plain);
                }
                NotchKind::V => add(at, depth, NotchShape::Plain),
                NotchKind::T => add(at, width, NotchShape::T),
                NotchKind::Castle => add(at, width, NotchShape::Castle),
                NotchKind::U => add(at, width, NotchShape::U),
            }
        }
        let grain = piece.grainline.or_else(|| pf_core::marks::default_grainline(&outline));
        // The fold: the seam line's fold edge, which the cutting line runs along.
        let mirror = piece.fold.and_then(|e| {
            let sp = path.subpaths.iter().find(|sp| sp.closed)?;
            (e < sp.segment_count()).then(|| {
                let seg = sp.segment(e);
                [[seg.p0.x, seg.p0.y], [seg.p3.x, seg.p3.y]]
            })
        });
        let mut internal = Vec::new();
        for c in children.iter().filter(|c| mark_of(c).is_none()) {
            internal_lines(c, &mut internal);
        }
        let text_at = grain.map(|g| (g.from + g.to) * 0.5).unwrap_or_else(|| outline.points.first().copied().unwrap_or_default());
        let (right, left) = if piece.mirror { (piece.cut - piece.cut / 2, piece.cut / 2) } else { (piece.cut, 0) };
        // Points (y down) for now; `to_mm` turns everything at once.
        let mm = |p: Pt| -> Mm { [p.x, p.y] };
        out.push(AamaPiece {
            name,
            quantity: (right, left),
            material: piece.fabric.clone(),
            boundary: cut.iter().map(|p| mm(*p)).collect(),
            sew_line: outline.points.iter().map(|p| mm(*p)).collect(),
            notches: notches
                .into_iter()
                .map(|(at, inward, depth, width, shape)| AamaNotch {
                    at: mm(at),
                    angle: (-inward.y).atan2(inward.x).to_degrees(),
                    depth,
                    width,
                    shape,
                })
                .collect(),
            grain: grain.map(|g| [mm(g.from), mm(g.to)]),
            mirror,
            internal: internal.iter().map(|l| l.iter().map(|p| mm(*p)).collect()).collect(),
            drills: piece.drills.iter().map(|d| AamaDrill { at: mm(d.at), diameter: mm_to_pt(d.diameter) }).collect(),
            text_at: mm(text_at),
        });
    }
    // Every piece name once in the file, as ASTM asks: a second "Front" is "Front 2".
    let mut seen: Vec<String> = Vec::new();
    for p in &mut out {
        let base = p.name.clone();
        let mut k = 2;
        while seen.iter().any(|s| s.eq_ignore_ascii_case(&p.name)) {
            p.name = format!("{base} {k}");
            k += 1;
        }
        seen.push(p.name.clone());
    }
    Ok(out)
}

/// The whole of a piece cut on the fold (one with a mirror line), in points, y down: its
/// cutting line opened out across the fold, and its sew line, notches and internal lines on both
/// halves; the piece as it is when it has no mirror line or can't be opened out.
pub(crate) fn opened(piece: &AamaPiece) -> AamaPiece {
    let Some([a, b]) = piece.mirror else { return piece.clone() };
    let (pa, pb) = (Pt::new(a[0], a[1]), Pt::new(b[0], b[1]));
    let pts = |v: &[Mm]| v.iter().map(|q| Pt::new(q[0], q[1])).collect::<Vec<_>>();
    let mm = |v: Vec<Pt>| v.into_iter().map(|q| [q.x, q.y]).collect::<Vec<Mm>>();
    let Some(boundary) = pf_core::marker::opened_out(&pts(&piece.boundary), pa, pb) else { return piece.clone() };
    let Some(d) = (pb - pa).unit() else { return piece.clone() };
    let flip = |q: Mm| {
        let v = Pt::new(q[0], q[1]) - pa;
        let r = pa + d * (2.0 * v.dot(d)) - v;
        [r.x, r.y]
    };
    let mut out = piece.clone();
    out.boundary = mm(boundary);
    out.mirror = None;
    if piece.sew_line.len() >= 3 {
        out.sew_line = pf_core::marker::opened_out(&pts(&piece.sew_line), pa, pb).map(mm).unwrap_or_else(|| piece.sew_line.clone());
    }
    for n in &piece.notches {
        // The direction turns over too: its angle is counter-clockwise with y up, y runs down.
        let (s, c) = n.angle.to_radians().sin_cos();
        let v = Pt::new(c, -s);
        let r = d * (2.0 * v.dot(d)) - v;
        out.notches.push(AamaNotch { at: flip(n.at), angle: (-r.y).atan2(r.x).to_degrees(), ..*n });
    }
    out.internal.extend(piece.internal.iter().map(|l| l.iter().map(|q| flip(*q)).collect()));
    out.drills.extend(piece.drills.iter().map(|d| AamaDrill { at: flip(d.at), ..*d }));
    // The fold itself, drawn inside the whole piece.
    out.internal.push(vec![a, b]);
    out
}

/// Points (y down, anywhere) to millimetres (y up), the pieces' bounds starting at x 0 and
/// [`TEXT_ROOM_MM`] above the style system text at the origin.
fn to_mm(pieces: &mut [AamaPiece]) {
    let (mut x0, mut y1) = (f64::INFINITY, f64::NEG_INFINITY);
    for p in pieces.iter() {
        for q in &p.boundary {
            x0 = x0.min(q[0]);
            y1 = y1.max(q[1]);
        }
    }
    let f = |q: &mut Mm| *q = [pt_to_mm(q[0] - x0), pt_to_mm(y1 - q[1]) + TEXT_ROOM_MM];
    for p in pieces.iter_mut() {
        p.boundary.iter_mut().for_each(f);
        p.sew_line.iter_mut().for_each(f);
        p.internal.iter_mut().flatten().for_each(f);
        if let Some(g) = &mut p.grain {
            g.iter_mut().for_each(f);
        }
        if let Some(m) = &mut p.mirror {
            m.iter_mut().for_each(f);
        }
        f(&mut p.text_at);
        for n in &mut p.notches {
            f(&mut n.at);
            n.depth = pt_to_mm(n.depth);
            n.width = pt_to_mm(n.width);
        }
        for d in &mut p.drills {
            f(&mut d.at);
            d.diameter = pt_to_mm(d.diameter);
        }
    }
}

pub(super) fn encode(doc: &Document, p: &Value) -> Result<Encoded> {
    let version = match p.get("version").and_then(Value::as_str).map(|v| v.trim().to_ascii_uppercase()) {
        None => Version::R13,
        Some(v) if v == "R13" || v == "AC1012" => Version::R13,
        Some(v) if v == "R12" || v == "AC1009" => Version::R12,
        Some(v) => return Err(bad(C, format!("version: R13 (ASTM D6673) or R12 for DXF-AAMA, not {v:?}"))),
    };
    let text = |k: &str, d: &str| p.get(k).and_then(Value::as_str).unwrap_or(d).to_string();
    let mut pieces = pieces(doc)?;
    if pieces.is_empty() {
        return Err(bad(C, "no pattern pieces to export: make the outlines pattern pieces first (Pattern › Make Pattern Piece)"));
    }
    to_mm(&mut pieces);
    let [y, mo, d, h, mi, _] = vectorcraft_doc::metadata::output_time().map(vectorcraft_doc::metadata::civil).unwrap_or([1970, 1, 1, 0, 0, 0]);
    let style = AamaStyle {
        name: text("styleName", "Pattern"),
        author: format!("StitchFoundry;PatternFoundry;{}", env!("CARGO_PKG_VERSION")),
        date: format!("{d:02}-{mo:02}-{y:04}"),
        time: format!("{h:02}:{mi:02}"),
        sample_size: text("sampleSize", ""),
        tolerance: (pt_to_mm(FLATTEN_PT) * 100.0).ceil() / 100.0,
        version,
        pieces,
    };
    let s = pf_aama::write(&style).map_err(|e| bad(C, e.to_string()))?;
    let mut enc = Encoded::one(s.into_bytes());
    if style.name.chars().count() > 20 {
        enc.warnings.push("the Style Name is longer than 20 characters: some cutting-room systems shorten it".into());
    }
    if let Some(n) = style.pieces.iter().find(|p| p.name.chars().count() > 20) {
        enc.warnings.push(format!("the piece name {:?} is longer than 20 characters: some cutting-room systems shorten it", n.name));
    }
    Ok(enc)
}
