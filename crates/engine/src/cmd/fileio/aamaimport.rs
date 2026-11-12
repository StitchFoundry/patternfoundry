//! Opening DXF-AAMA files (PF-7.8): a `.dxf` written to ASTM D6673 (its pieces as blocks with
//! "Piece Name:" text, [`pf_aama::read`]) opens as pattern pieces, not as plain drawing. Each piece's
//! sew line becomes its seam line (its cutting line when the file has none), and each edge's seam
//! allowance is how far the cutting line is from the edge's middle: the piece's allowance is the
//! middle value, and edges that differ keep their own. The notches, grainline, mirror line (the fold),
//! drill holes, internal lines, quantity and material come along. Millimetres, y up in the file;
//! points, y down in the document, the pieces where they were relative to each other.

use std::sync::Arc;

use pf_aama::{Mm, NotchShape, Style};
use pf_core::{Grainline, Notch, NotchKind, Piece, Pt, mm_to_pt};
use vectorcraft_color::{Color, Paint};
use vectorcraft_doc::{Appearance, Document, LayerColor, Node, Unit};
use vectorcraft_geom::{PathData, Point, SubPath};

use super::super::*;
use super::err;

/// Room round the pieces on the page, in mm.
const MARGIN_MM: f64 = 20.0;
/// How far an edge's allowance may be from the piece's (mm) and still be the piece's.
const SAME_MM: f64 = 0.05;
/// How near the mirror line a seam edge's ends must be (mm) to be the fold.
const ON_FOLD_MM: f64 = 0.5;

/// Whether `bytes` is a DXF-AAMA file: piece system text, and pieces that read.
pub(super) fn is_aama(bytes: &[u8]) -> bool {
    bytes.windows(11).any(|w| w.eq_ignore_ascii_case(b"Piece Name:")) && pf_aama::read(bytes).is_ok()
}

/// How far `q` is from the segment `a`–`b`, and how far along it (0 to 1) its nearest point is.
fn to_segment(q: Mm, a: Mm, b: Mm) -> (f64, f64) {
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let len2 = dx * dx + dy * dy;
    let t = if len2 > 0.0 { (((q[0] - a[0]) * dx + (q[1] - a[1]) * dy) / len2).clamp(0.0, 1.0) } else { 0.0 };
    let (px, py) = (a[0] + dx * t, a[1] + dy * t);
    (((q[0] - px).powi(2) + (q[1] - py).powi(2)).sqrt(), t)
}

/// The edge of the closed ring `ring` nearest `q`: (edge, along it 0 to 1, distance).
fn nearest_edge(ring: &[Mm], q: Mm) -> Option<(usize, f64, f64)> {
    (0..ring.len())
        .map(|i| {
            let (d, t) = to_segment(q, ring[i], ring[(i + 1) % ring.len()]);
            (i, t, d)
        })
        .min_by(|a, b| a.2.total_cmp(&b.2))
}

/// The edges of a closed ring in a uniform grid, to find how far many points are from the ring
/// without measuring every edge each time: a densely digitised piece has tens of thousands of
/// edges on both its lines, and checking each pair froze the app for minutes (issue 169).
struct EdgeGrid<'a> {
    ring: &'a [Mm],
    origin: Mm,
    cell: f64,
    cols: i64,
    rows: i64,
    /// The edges with a sample point in each cell, row by row.
    cells: Vec<Vec<usize>>,
}

impl<'a> EdgeGrid<'a> {
    fn new(ring: &'a [Mm]) -> Self {
        let n = ring.len().max(1);
        let (mut lo, mut hi) = ([f64::INFINITY; 2], [f64::NEG_INFINITY; 2]);
        let mut perimeter = 0.0;
        for (i, p) in ring.iter().enumerate() {
            (lo, hi) = ([lo[0].min(p[0]), lo[1].min(p[1])], [hi[0].max(p[0]), hi[1].max(p[1])]);
            let q = ring[(i + 1) % ring.len()];
            perimeter += (q[0] - p[0]).hypot(q[1] - p[1]);
        }
        if ring.is_empty() {
            (lo, hi) = ([0.0; 2], [0.0; 2]);
        }
        // About as many cells as edges, each holding a few edges.
        let extent = (hi[0] - lo[0]).max(hi[1] - lo[1]);
        let cell = (2.0 * perimeter / n as f64).max(extent / (n as f64).sqrt());
        let cell = if cell.is_finite() && cell > 0.0 { cell } else { 1.0 };
        let span = |a: f64, b: f64| ((b - a) / cell).floor().clamp(0.0, 1e6) as i64 + 1;
        let (cols, rows) = (span(lo[0], hi[0]), span(lo[1], hi[1]));
        let mut grid = EdgeGrid { ring, origin: lo, cell, cols, rows, cells: vec![Vec::new(); (cols * rows) as usize] };
        // Each edge sampled every half cell, so every point on it is within a quarter cell of a
        // sample, and so of a cell that lists it.
        for (i, &a) in ring.iter().enumerate() {
            let b = ring[(i + 1) % ring.len()];
            let steps = ((b[0] - a[0]).hypot(b[1] - a[1]) / (cell / 2.0)).ceil().clamp(0.0, 1e7) as usize;
            for k in 0..=steps {
                let t = if steps == 0 { 0.0 } else { k as f64 / steps as f64 };
                let (cx, cy) = grid.key([a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t]);
                if let Some(c) = grid.index(cx, cy)
                    && grid.cells[c].last() != Some(&i)
                {
                    grid.cells[c].push(i);
                }
            }
        }
        grid
    }

    fn key(&self, p: Mm) -> (i64, i64) {
        (((p[0] - self.origin[0]) / self.cell).floor() as i64, ((p[1] - self.origin[1]) / self.cell).floor() as i64)
    }

    fn index(&self, cx: i64, cy: i64) -> Option<usize> {
        ((0..self.cols).contains(&cx) && (0..self.rows).contains(&cy)).then(|| (cy * self.cols + cx) as usize)
    }

    /// How far `q` is from the ring: the distance of [`nearest_edge`], found from the cells round
    /// `q` outward.
    fn distance(&self, q: Mm) -> Option<f64> {
        if self.ring.is_empty() {
            return None;
        }
        let (cx, cy) = self.key(q);
        let n = self.ring.len();
        // From well outside the grid, searching outward costs more than measuring every edge.
        let far = cx < -self.cols || cx > 2 * self.cols || cy < -self.rows || cy > 2 * self.rows;
        if far {
            return nearest_edge(self.ring, q).map(|e| e.2);
        }
        let last = (cx.max(self.cols - 1 - cx)).max(cy.max(self.rows - 1 - cy)).max(0);
        let mut best = f64::INFINITY;
        let visit = |x: i64, y: i64, best: &mut f64| {
            for &i in self.index(x, y).map_or(&[][..], |c| &self.cells[c]) {
                *best = best.min(to_segment(q, self.ring[i], self.ring[(i + 1) % n]).0);
            }
        };
        for r in 0..=last {
            // A sample in ring `r` (cells `r` from q's) is at least `r - 1` cells from q, and the
            // edge it stands for at most a quarter cell nearer: nothing from here on is closer.
            if (r - 1) as f64 * self.cell - self.cell / 4.0 >= best {
                break;
            }
            if r == 0 {
                visit(cx, cy, &mut best);
                continue;
            }
            for x in (cx - r).max(-1)..=(cx + r).min(self.cols) {
                visit(x, cy - r, &mut best);
                visit(x, cy + r, &mut best);
            }
            for y in (cy - r + 1).max(-1)..=(cy + r - 1).min(self.rows) {
                visit(cx - r, y, &mut best);
                visit(cx + r, y, &mut best);
            }
        }
        Some(best)
    }
}

/// How far a point `q` is from the infinite line through `a` and `b`.
fn to_line(q: Mm, a: Mm, b: Mm) -> f64 {
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let len = (dx * dx + dy * dy).sqrt();
    if len == 0.0 {
        return ((q[0] - a[0]).powi(2) + (q[1] - a[1]).powi(2)).sqrt();
    }
    ((q[0] - a[0]) * dy - (q[1] - a[1]) * dx).abs() / len
}

/// The edges of the closed ring `ring` that are a dart's legs: two edges meeting in a sharp inward
/// corner, as [`pf_core::cutting_line`] finds darts.
fn dart_legs(ring: &[Mm]) -> Vec<usize> {
    let n = ring.len();
    let area: f64 = (0..n).map(|i| ring[i][0] * ring[(i + 1) % n][1] - ring[(i + 1) % n][0] * ring[i][1]).sum();
    let dir = |i: usize| {
        let (p, q) = (ring[i], ring[(i + 1) % n]);
        let len = (q[0] - p[0]).hypot(q[1] - p[1]);
        if len > 0.0 { [(q[0] - p[0]) / len, (q[1] - p[1]) / len] } else { [0.0, 0.0] }
    };
    let mut legs = Vec::new();
    if n < 5 {
        return legs;
    }
    let sharp = pf_core::allowance::DART_DEGREES.to_radians().cos();
    for i in 0..n {
        let (a, b) = (dir(i), dir((i + 1) % n));
        let turn = a[0] * b[1] - a[1] * b[0];
        if turn * area.signum() < 0.0 && -(a[0] * b[0] + a[1] * b[1]) > sharp {
            legs.extend([i, (i + 1) % n]);
        }
    }
    legs
}

/// The style's pieces as a document.
pub(super) fn import(bytes: &[u8]) -> Result<(Document, Vec<String>)> {
    let style: Style = pf_aama::read(bytes).map_err(|e| err(format!("can't open the DXF-AAMA file: {e}")))?;
    let (mut x0, mut y0, mut x1, mut y1) = (f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);
    for q in style.pieces.iter().flat_map(|p| &p.boundary) {
        (x0, y0, x1, y1) = (x0.min(q[0]), y0.min(q[1]), x1.max(q[0]), y1.max(q[1]));
    }
    if !(x0.is_finite() && y0.is_finite() && x1.is_finite() && y1.is_finite()) || (x1 - x0).max(y1 - y0) > 100_000.0 {
        return Err(err("the DXF-AAMA file's pieces have no usable size"));
    }
    // File millimetres (y up) to document points (y down), the pieces a margin in from the page.
    let pt = |q: Mm| Pt::new(mm_to_pt(q[0] - x0 + MARGIN_MM), mm_to_pt(y1 - q[1] + MARGIN_MM));
    let point = |q: Mm| {
        let p = pt(q);
        Point::new(p.x, p.y)
    };
    let mut d = Document::new(mm_to_pt(x1 - x0 + 2.0 * MARGIN_MM), mm_to_pt(y1 - y0 + 2.0 * MARGIN_MM));
    d.units = Unit::Millimeters;
    d.layers.clear();
    let mut warnings = Vec::new();
    let mut groups = Vec::new();
    for ap in &style.pieces {
        let name = ap.name.trim().to_string();
        let sewn = ap.sew_line.len() >= 3;
        let seam = if sewn { &ap.sew_line } else { &ap.boundary };
        let edge = |i: usize| (seam[i], seam[(i + 1) % seam.len()]);
        // The fold: the longest seam edge lying on the mirror line.
        let fold = ap.mirror.and_then(|[a, b]| {
            let fold =
                (0..seam.len()).filter(|&i| to_line(edge(i).0, a, b) <= ON_FOLD_MM && to_line(edge(i).1, a, b) <= ON_FOLD_MM).max_by(|&i, &j| {
                    let len = |e: usize| {
                        let (p, q) = edge(e);
                        (q[0] - p[0]).hypot(q[1] - p[1])
                    };
                    len(i).total_cmp(&len(j))
                });
            if fold.is_none() {
                warnings.push(format!("{name}: its mirror line is not along its seam line, so it is not cut on the fold here"));
            }
            fold
        });
        // Each edge's allowance: how far the cutting line is from the edge's middle. The piece's
        // allowance is the middle value; edges further from it than a twentieth of a millimetre
        // keep their own. A dart's legs are left out: the cutting line bridges darts.
        let mut own = std::collections::BTreeMap::new();
        let mut allowance = 0.0;
        if sewn {
            let round = |v: f64| (v * 100.0).round() / 100.0;
            let legs: std::collections::HashSet<usize> = dart_legs(seam).into_iter().collect();
            let cutting = EdgeGrid::new(&ap.boundary);
            let gaps: Vec<(usize, f64)> = (0..seam.len())
                .filter(|i| Some(*i) != fold && !legs.contains(i))
                .filter_map(|i| {
                    let (p, q) = edge(i);
                    cutting.distance([(p[0] + q[0]) / 2.0, (p[1] + q[1]) / 2.0]).map(|d| (i, round(d)))
                })
                .collect();
            let mut sorted: Vec<f64> = gaps.iter().map(|g| g.1).collect();
            sorted.sort_by(f64::total_cmp);
            allowance = sorted.get(sorted.len() / 2).copied().unwrap_or_default();
            for (i, g) in gaps {
                if (g - allowance).abs() > SAME_MM {
                    own.insert(i, pf_core::piece::check_allowance(g).unwrap_or(pf_core::piece::MAX_ALLOWANCE_MM));
                }
            }
        }
        let allowance = pf_core::piece::check_allowance(allowance).unwrap_or(pf_core::piece::MAX_ALLOWANCE_MM);
        let (right, left) = ap.quantity;
        let mut piece = Piece {
            name: pf_core::piece::check_name(&name).unwrap_or_default(),
            cut: right.saturating_add(left).clamp(1, pf_core::piece::MAX_CUT),
            mirror: left > 0,
            fabric: ap.material.trim().to_string(),
            allowance,
            edges: own,
            fold,
            ..Piece::default()
        };
        for n in &ap.notches {
            // On the cutting line in the file; on the nearest seam edge here.
            let Some((edge, at, _)) = nearest_edge(seam, n.at) else { continue };
            let kind = match n.shape {
                NotchShape::Plain if n.width > 0.0 => NotchKind::V,
                NotchShape::Plain => NotchKind::Slit,
                NotchShape::T => NotchKind::T,
                NotchShape::Castle => NotchKind::Castle,
                NotchShape::U => NotchKind::U,
            };
            if piece.add_notch(Notch { edge, at, kind }).is_err() {
                warnings.push(format!("{name}: only the first {} notches were read", pf_core::piece::MAX_NOTCHES));
                break;
            }
        }
        if let Some([a, b]) = ap.grain {
            piece.grainline = Some(Grainline { from: pt(a), to: pt(b) });
        }
        for dr in &ap.drills {
            let _ = piece.add_drill(pf_core::Drill { at: pt(dr.at), diameter: dr.diameter.clamp(0.0, pf_core::piece::MAX_DRILL_MM) });
        }
        let seam_node = Node::path(
            d.alloc_id(),
            PathData::single(SubPath::polyline(&seam.iter().map(|q| point(*q)).collect::<Vec<_>>(), true)),
            Appearance::default(),
        );
        let ink = Appearance::basic(Paint::None, Paint::solid(Color::BLACK), 0.75);
        let inside: Vec<Node> = ap
            .internal
            .iter()
            .filter(|l| l.len() >= 2)
            .map(|l| {
                Node::path(d.alloc_id(), PathData::single(SubPath::polyline(&l.iter().map(|q| point(*q)).collect::<Vec<_>>(), false)), ink.clone())
            })
            .collect();
        let gid = d.alloc_id();
        groups.push(super::super::piece::new_piece(gid, seam_node, piece, inside));
    }
    let ids: Vec<_> = groups.iter().map(|g| g.id).collect();
    let mut layer = Node::layer(d.alloc_id(), "Pattern pieces", LayerColor::Preset(1));
    if let Some(ch) = layer.children_mut() {
        *ch = groups.into_iter().map(Arc::new).collect();
    }
    d.layers.push(Arc::new(layer));
    d.pieces = true;
    for id in ids {
        super::super::piece::rebuild(&mut d, id)?;
    }
    if !style.sample_size.trim().is_empty() {
        warnings.push(format!("drafted in size {}; the file's grade rules are not read", style.sample_size.trim()));
    }
    Ok((d, warnings))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A small deterministic generator (no new dependency).
    fn lcg(seed: &mut u64) -> f64 {
        *seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
        (*seed >> 11) as f64 / (1u64 << 53) as f64
    }

    #[test]
    fn the_edge_grid_measures_the_same_distance_as_every_edge() {
        let mut seed = 169;
        let round: Vec<Mm> = (0..500)
            .map(|i| {
                let a = f64::from(i) / 500.0 * std::f64::consts::TAU;
                [300.0 * a.cos(), 200.0 * a.sin()]
            })
            .collect();
        let zigzag: Vec<Mm> = (0..300).map(|i| [f64::from(i % 2) * 400.0, f64::from(i)]).collect();
        let random: Vec<Mm> = (0..200).map(|_| [lcg(&mut seed) * 1000.0, lcg(&mut seed) * 50.0]).collect();
        let few: Vec<Mm> = vec![[0.0, 0.0], [1000.0, 0.0], [1000.0, 0.001]];
        let one: Vec<Mm> = vec![[5.0, 5.0]];
        for ring in [&round, &zigzag, &random, &few, &one] {
            let grid = EdgeGrid::new(ring);
            let mut points: Vec<Mm> = (0..400).map(|_| [lcg(&mut seed) * 1400.0 - 400.0, lcg(&mut seed) * 1400.0 - 400.0]).collect();
            points.extend([[1e300, -1e300], [5.0, 5.0], [0.0, 0.0], [-1e6, 3.0]]);
            for q in points {
                let want = nearest_edge(ring, q).map(|e| e.2);
                let got = grid.distance(q);
                assert_eq!(got, want, "{q:?} on a ring of {}", ring.len());
            }
        }
        assert_eq!(EdgeGrid::new(&[]).distance([0.0, 0.0]), None);
    }
}
