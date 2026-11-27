//! Marker making (PF-9): every piece to cut laid on a length of fabric of a given width, as
//! closely as it goes, so the marker (the cutting layout) is as short as it can be.
//!
//! The fabric runs along x (its length) from x = 0, its width along y from 0 to `width`. Each
//! [`Part`] comes already turned so its grainline runs along x, as the grain must run along the
//! fabric's length; a part may also be laid turned half round (180°) unless the fabric has a nap
//! or a one-way print. Parts are laid largest first (those on the fold before the rest), each
//! where its right end comes furthest left (then nearest the top edge): a *bottom-left fill*, the
//! classic greedy heuristic for irregular nesting (Hopper & Turton, "An empirical investigation
//! of meta-heuristic and heuristic algorithms for a 2D packing problem", EJOR 128 (2001)). Overlap is tested on a grid
//! of `step` cells: each part covers every cell its outline, grown by half the gap, touches, so
//! parts laid never overlap and are at least `gap` apart; a finer step lays them closer, at more
//! cost.
//!
//! On fabric folded lengthwise the fold is the top edge (y = 0): a part cut on the fold
//! ([`Part::fold`]) lies with its fold line against it, and [`opened_out`] makes the whole of such
//! a part for fabric laid out single.
//!
//! Every length is in the parts' own unit.

use crate::Pt;

/// Most parts one marker lays.
pub const MAX_PARTS: usize = 5_000;
/// Most grid cells a marker uses (fabric rows × columns): a bound on memory and time.
pub const MAX_CELLS: usize = 40_000_000;
/// Most points one part's outline has.
pub const MAX_POINTS: usize = 100_000;

/// A part to lay: its outline (a polygon, grainline along x), whether it may be turned 180°,
/// and whether it is cut on the fold: its smallest y on the fold line, so it lies against the
/// top edge (never turned).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Part {
    pub points: Vec<Pt>,
    pub turn: bool,
    pub fold: bool,
}

/// Where a part went: its points, turned half round about the origin when `turned`, moved by
/// `offset`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Placed {
    pub offset: Pt,
    pub turned: bool,
}

impl Placed {
    /// Where point `p` of the part lies on the fabric.
    pub fn apply(&self, p: Pt) -> Pt {
        let q = if self.turned { Pt::new(-p.x, -p.y) } else { p };
        q + self.offset
    }
}

/// A laid marker.
#[derive(Clone, Debug, PartialEq)]
pub struct Layout {
    /// Each part's place, in the order given; `None` for a part wider than the fabric.
    pub placed: Vec<Option<Placed>>,
    /// The marker's length: from x = 0 to the furthest point laid.
    pub length: f64,
}

/// Why a marker can't be laid.
#[derive(Clone, Debug, PartialEq)]
pub enum MarkerError {
    /// A width not more than 0, or not a number.
    Width,
    /// A gap below 0, or not a number.
    Gap,
    /// A grid step not more than 0, or not a number.
    Step,
    /// More than [`MAX_PARTS`] parts, or a part with too many points.
    TooMany,
    /// The grid would take more than [`MAX_CELLS`] cells.
    TooLarge,
    /// A plaid's repeat not more than 0 each way, or not a number.
    Repeat,
}

impl std::fmt::Display for MarkerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            MarkerError::Width => "the fabric's width must be more than 0",
            MarkerError::Gap => "the gap must be 0 or more",
            MarkerError::Step => "the precision must be more than 0",
            MarkerError::TooMany => "too many pieces to lay at once",
            MarkerError::TooLarge => "the marker is too large at this precision: give a coarser one",
            MarkerError::Repeat => "the plaid's repeat must be more than 0 along and across",
        })
    }
}

impl std::error::Error for MarkerError {}

/// The area inside polygon `points` (positive whichever way it runs).
pub fn area(points: &[Pt]) -> f64 {
    let n = points.len();
    (0..n).map(|i| points[i].cross(points[(i + 1) % n])).sum::<f64>().abs() / 2.0
}

/// Whether point `q` is inside polygon `pts` (even-odd).
fn inside(pts: &[Pt], q: Pt) -> bool {
    let n = pts.len();
    let mut c = false;
    for i in 0..n {
        let (a, b) = (pts[i], pts[(i + 1) % n]);
        if (a.y > q.y) != (b.y > q.y) && q.x < a.x + (q.y - a.y) / (b.y - a.y) * (b.x - a.x) {
            c = !c;
        }
    }
    c
}

/// Whether segments `a0`–`a1` and `b0`–`b1` cross (touching at an end counts as not crossing).
fn crosses(a0: Pt, a1: Pt, b0: Pt, b1: Pt) -> bool {
    let (da, db) = (a1 - a0, b1 - b0);
    let den = da.cross(db);
    if den.abs() < 1e-12 {
        return false;
    }
    let t = (b0 - a0).cross(db) / den;
    let u = (b0 - a0).cross(da) / den;
    let e = 1e-9;
    t > e && t < 1.0 - e && u > e && u < 1.0 - e
}

/// Whether polygons `a` and `b` overlap: their outlines cross, or one lies inside the other
/// (pieces laid by hand on a marker, PF-9.3). Outlines that only touch don't overlap.
pub fn overlaps(a: &[Pt], b: &[Pt]) -> bool {
    if a.len() < 3 || b.len() < 3 {
        return false;
    }
    let bounds = |p: &[Pt]| {
        p.iter().fold((f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY), |(x0, y0, x1, y1), q| {
            (x0.min(q.x), y0.min(q.y), x1.max(q.x), y1.max(q.y))
        })
    };
    let (ba, bb) = (bounds(a), bounds(b));
    if ba.2 <= bb.0 || bb.2 <= ba.0 || ba.3 <= bb.1 || bb.3 <= ba.1 {
        return false;
    }
    let (na, nb) = (a.len(), b.len());
    for i in 0..na {
        for j in 0..nb {
            if crosses(a[i], a[(i + 1) % na], b[j], b[(j + 1) % nb]) {
                return true;
            }
        }
    }
    // No crossing: one inside the other, or apart. A vertex on the other's outline is ambiguous,
    // so test each polygon's middle-most vertex: its centroid when that is inside it.
    let probe = |p: &[Pt]| {
        let c = p.iter().fold(Pt::new(0.0, 0.0), |s, q| s + *q) * (1.0 / p.len() as f64);
        if inside(p, c) { c } else { p[0] }
    };
    inside(b, probe(a)) || inside(a, probe(b))
}

/// A part's cells: per grid row, from its top, half-open runs of columns.
struct Cells {
    cols: usize,
    rows: Vec<Vec<(usize, usize)>>,
}

/// The x-ranges where polygon `pts` meets the band `y0..=y1`: each edge's x-range in the band,
/// and the inside of the polygon along the band's middle line (a point of the polygon in the
/// band either has the outline above or below it in the band, or the whole height of the band
/// inside the polygon there, so this is all of it).
fn band(pts: &[Pt], y0: f64, y1: f64) -> Vec<(f64, f64)> {
    let n = pts.len();
    let mut out = Vec::new();
    for i in 0..n {
        let (p, q) = (pts[i], pts[(i + 1) % n]);
        let (lo, hi) = (p.y.min(q.y), p.y.max(q.y));
        if hi < y0 || lo > y1 {
            continue;
        }
        let x_at = |y: f64| if (q.y - p.y).abs() < 1e-12 { p.x } else { p.x + (q.x - p.x) * (y - p.y) / (q.y - p.y) };
        let (a, b) = (x_at(lo.max(y0)), x_at(hi.min(y1)));
        let (a, b) = if (q.y - p.y).abs() < 1e-12 { (p.x, q.x) } else { (a, b) };
        out.push((a.min(b), a.max(b)));
    }
    let y = (y0 + y1) / 2.0;
    let mut xs: Vec<f64> = (0..n)
        .filter_map(|i| {
            let (p, q) = (pts[i], pts[(i + 1) % n]);
            ((p.y <= y) != (q.y <= y)).then(|| p.x + (q.x - p.x) * (y - p.y) / (q.y - p.y))
        })
        .collect();
    xs.sort_by(f64::total_cmp);
    out.extend(xs.chunks_exact(2).map(|c| (c[0], c[1])));
    out
}

/// The cells polygon `pts` (with its smallest x and y at `pad`) covers, grown by `pad`.
fn cells(pts: &[Pt], pad: f64, step: f64) -> Cells {
    let (w, h) = pts.iter().fold((0.0f64, 0.0f64), |(w, h), p| (w.max(p.x), h.max(p.y)));
    let cols = ((w + pad) / step).ceil().max(1.0) as usize;
    let n_rows = ((h + pad) / step).ceil().max(1.0) as usize;
    let mut rows = Vec::with_capacity(n_rows);
    for j in 0..n_rows {
        let (y0, y1) = (j as f64 * step - pad, (j + 1) as f64 * step + pad);
        let mut runs: Vec<(usize, usize)> = band(pts, y0, y1)
            .into_iter()
            .map(|(a, b)| (((a - pad) / step).floor().max(0.0) as usize, (((b + pad) / step).ceil() as usize).clamp(1, cols)))
            .filter(|(a, b)| a < b)
            .collect();
        runs.sort_unstable();
        let mut merged: Vec<(usize, usize)> = Vec::new();
        for (a, b) in runs {
            match merged.last_mut() {
                Some(last) if a <= last.1 => last.1 = last.1.max(b),
                _ => merged.push((a, b)),
            }
        }
        rows.push(merged);
    }
    Cells { cols, rows }
}

/// The fabric's grid: a bit per cell, set where a part lies.
struct Grid {
    words: usize,
    bits: Vec<u64>,
}

impl Grid {
    fn mask(from: usize, to: usize) -> impl Iterator<Item = (usize, u64)> {
        let (first, last) = (from / 64, (to - 1) / 64);
        (first..=last).map(move |w| {
            let lo = if w == first { from % 64 } else { 0 };
            let hi = if w == last { (to - 1) % 64 + 1 } else { 64 };
            let m = if hi - lo == 64 { u64::MAX } else { ((1u64 << (hi - lo)) - 1) << lo };
            (w, m)
        })
    }
    fn free(&self, row: usize, from: usize, to: usize) -> bool {
        let base = row * self.words;
        Self::mask(from, to).all(|(w, m)| self.bits[base + w] & m == 0)
    }
    fn fill(&mut self, row: usize, from: usize, to: usize) {
        let base = row * self.words;
        for (w, m) in Self::mask(from, to) {
            self.bits[base + w] |= m;
        }
    }
    fn fits(&self, c: &Cells, cx: usize, cy: usize) -> bool {
        c.rows.iter().enumerate().all(|(r, runs)| runs.iter().all(|&(a, b)| self.free(cy + r, cx + a, cx + b)))
    }
    fn put(&mut self, c: &Cells, cx: usize, cy: usize) {
        for (r, runs) in c.rows.iter().enumerate() {
            for &(a, b) in runs {
                self.fill(cy + r, cx + a, cx + b);
            }
        }
    }
}

/// One way a part can lie: its cells, and how its points map into them.
struct Way {
    cells: Cells,
    turned: bool,
    /// What moves the (turned) points so their smallest x and y are at the pad.
    shift: Pt,
}

fn way(points: &[Pt], turned: bool, pad: f64, step: f64) -> Way {
    let pts: Vec<Pt> = points.iter().map(|p| if turned { Pt::new(-p.x, -p.y) } else { *p }).collect();
    let (mx, my) = pts.iter().fold((f64::INFINITY, f64::INFINITY), |(x, y), p| (x.min(p.x), y.min(p.y)));
    let shift = Pt::new(pad - mx, pad - my);
    let moved: Vec<Pt> = pts.iter().map(|p| *p + shift).collect();
    Way { cells: cells(&moved, pad, step), turned, shift }
}

/// Lay `parts` across fabric `width` wide, `gap` apart, testing overlap on a grid of `step`.
pub fn lay(parts: &[Part], width: f64, gap: f64, step: f64) -> Result<Layout, MarkerError> {
    lay_matched(parts, width, gap, step, None)
}

/// [`lay`], on a plaid or check whose design repeats every `repeat.x` along the fabric and
/// `repeat.y` across it (PF-9.4): each part is moved only by whole repeats from where its points
/// are, so parts placed in step with each other (pieces drawn level with each other, matching at
/// their seams) meet the same lines of the plaid on the fabric, as plaids are matched at their
/// seam lines (Seamwork, "10 tips for matching plaids and stripes",
/// <https://seamwork.com/articles/10-tips-for-matching-plaids-and-stripes>). Parts are never
/// turned (the plaid would turn with them). A part on the fold keeps against the fold and is
/// matched along the fabric only.
pub fn lay_matched(parts: &[Part], width: f64, gap: f64, step: f64, repeat: Option<Pt>) -> Result<Layout, MarkerError> {
    if !(width.is_finite() && width > 0.0) {
        return Err(MarkerError::Width);
    }
    if !(gap.is_finite() && gap >= 0.0) {
        return Err(MarkerError::Gap);
    }
    if !(step.is_finite() && step > 0.0) {
        return Err(MarkerError::Step);
    }
    if parts.len() > MAX_PARTS || parts.iter().any(|p| p.points.len() > MAX_POINTS) {
        return Err(MarkerError::TooMany);
    }
    if repeat.is_some_and(|r| !(r.is_finite() && r.x > 0.0 && r.y > 0.0)) {
        return Err(MarkerError::Repeat);
    }
    let pad = gap / 2.0;
    // On a plaid a part lies off the grid by up to half a cell: its cells are grown by a whole one.
    let grow = if repeat.is_some() { step } else { 0.0 };
    // The grid starts half a gap before the fabric's top edge and its start, and ends half a gap
    // past its bottom edge: the gap is kept between parts, not from the edges.
    let fabric_rows = ((width + gap) / step).floor() as usize;
    let mut placed = vec![None; parts.len()];
    // Outlines that are no polygon, or not finite, are not laid.
    let usable = |p: &Part| p.points.len() >= 3 && p.points.iter().all(|q| q.is_finite());
    let ways: Vec<Vec<Way>> = parts
        .iter()
        .map(|p| {
            if !usable(p) {
                return Vec::new();
            }
            // A part with more rows than the fabric can't be laid: its grid isn't made (a huge
            // part would take more memory than there is).
            let (y0, y1) = p.points.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), q| (a.min(q.y), b.max(q.y)));
            if (y1 - y0 + 2.0 * (pad + grow)) / step > fabric_rows as f64 {
                return Vec::new();
            }
            let mut w = vec![way(&p.points, false, pad + grow, step)];
            if p.turn && !p.fold && repeat.is_none() {
                w.push(way(&p.points, true, pad + grow, step));
            }
            w
        })
        .collect();
    // The grid is as long as every part laid end to end.
    let total: usize = ways.iter().filter_map(|w| w.first()).fold(1usize, |t, w| t.saturating_add(w.cells.cols));
    if fabric_rows.saturating_mul(total) > MAX_CELLS {
        return Err(MarkerError::TooLarge);
    }
    let words = total.div_ceil(64);
    let mut grid = Grid { words, bits: vec![0; words * fabric_rows] };
    let mut order: Vec<usize> = (0..parts.len()).collect();
    // Parts on the fold first (they have only the fold to go along), then largest first.
    order.sort_by(|&a, &b| parts[b].fold.cmp(&parts[a].fold).then(area(&parts[b].points).total_cmp(&area(&parts[a].points))).then(a.cmp(&b)));
    let mut length = 0.0f64;
    for i in order {
        // The way whose right end comes furthest left, then nearest the top.
        let mut best: Option<(usize, usize, usize)> = None;
        for (k, w) in ways[i].iter().enumerate() {
            let c = &w.cells;
            if c.rows.len() > fabric_rows || c.cols > total {
                continue;
            }
            // A part on the fold lies against the top edge only.
            let rows = if parts[i].fold { 0 } else { fabric_rows - c.rows.len() };
            let found = match repeat {
                None => (0..=total - c.cols).find_map(|cx| (0..=rows).find(|&cy| grid.fits(c, cx, cy)).map(|cy| (cx, cy))),
                // Whole repeats from where it is: the cell nearest each such place.
                Some(r) => {
                    let cell = |o: f64, shift: f64| ((o - shift + pad) / step).round();
                    let (k0, k1) =
                        (((w.shift.x - pad) / r.x).ceil() as i64, ((w.shift.x - pad + (total - c.cols) as f64 * step) / r.x).floor() as i64);
                    let (j0, j1) = (((w.shift.y - pad) / r.y).ceil() as i64, ((w.shift.y - pad + rows as f64 * step) / r.y).floor() as i64);
                    (k0..=k1).find_map(|k| {
                        let cx = cell(k as f64 * r.x, w.shift.x);
                        let rows_at: Vec<f64> = if parts[i].fold { vec![0.0] } else { (j0..=j1).map(|j| cell(j as f64 * r.y, w.shift.y)).collect() };
                        rows_at
                            .into_iter()
                            .filter(|cy| cx >= 0.0 && *cy >= 0.0 && cx as usize + c.cols <= total && *cy as usize <= rows)
                            .find(|cy| grid.fits(c, cx as usize, *cy as usize))
                            .map(|cy| (cx as usize, cy as usize))
                    })
                }
            };
            if let Some((cx, cy)) = found {
                let key = (cx + c.cols, cy);
                if best.is_none_or(|(bk, bx, by)| key < (bx + ways[i][bk].cells.cols, by)) {
                    best = Some((k, cx, cy));
                }
            }
        }
        let Some((k, cx, cy)) = best else { continue };
        let w = &ways[i][k];
        grid.put(&w.cells, cx, cy);
        let mut at = Placed { offset: w.shift + Pt::new(cx as f64 * step - pad, cy as f64 * step - pad), turned: w.turned };
        // On a plaid, exactly whole repeats from where it was (the cells were only the nearest).
        if let Some(r) = repeat {
            at.offset.x = (at.offset.x / r.x).round() * r.x;
            if !parts[i].fold {
                at.offset.y = (at.offset.y / r.y).round() * r.y;
            }
        }
        length = parts[i].points.iter().map(|p| at.apply(*p).x).fold(length, f64::max);
        placed[i] = Some(at);
    }
    Ok(Layout { placed, length })
}

/// The whole of a part cut on the fold: outline `points` (a polygon on one side of the fold line
/// through `a` and `b`, touching it along its fold edge) and its mirror image across the line,
/// as one polygon. `None` when the outline crosses the line or doesn't touch it.
pub fn opened_out(points: &[Pt], a: Pt, b: Pt) -> Option<Vec<Pt>> {
    let d = (b - a).unit()?;
    let n = points.len();
    if n < 3 {
        return None;
    }
    let size = points.iter().map(|p| (*p - a).len()).fold(0.0, f64::max);
    let eps = 1e-6 * size.max(1.0);
    let side = |p: Pt| d.cross(p - a);
    let on = |i: usize| side(points[i % n]).abs() <= eps;
    let off: Vec<f64> = points.iter().map(|p| side(*p)).filter(|v| v.abs() > eps).collect();
    if off.is_empty() || !(off.iter().all(|v| *v > 0.0) || off.iter().all(|v| *v < 0.0)) {
        return None;
    }
    // The runs of points on the line; the fold edge is the longest.
    let mut best: Option<(usize, usize, f64)> = None;
    for start in (0..n).filter(|&i| on(i) && !on(i + n - 1)) {
        let mut end = start;
        while on(end + 1) && end + 1 < start + n {
            end += 1;
        }
        let span = (points[end % n] - points[start]).len();
        if best.is_none_or(|(_, _, s)| span > s) {
            best = Some((start, end % n, span));
        }
    }
    let (first, last, _) = best?;
    // From the fold edge's end round the outline to its start, then back on the mirror side.
    let mut chain = Vec::new();
    let mut i = last;
    loop {
        chain.push(points[i]);
        if i == first {
            break;
        }
        i = (i + 1) % n;
    }
    let mirror = |p: Pt| {
        let v = p - a;
        a + d * (2.0 * v.dot(d)) - v
    };
    let inner: Vec<Pt> = chain[1..chain.len() - 1].iter().rev().map(|p| mirror(*p)).collect();
    chain.extend(inner);
    Some(chain)
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_huge_part_is_refused_at_once_not_gridded() {
        let rect =
            |w: f64, h: f64| Part { points: vec![Pt::new(0.0, 0.0), Pt::new(w, 0.0), Pt::new(w, h), Pt::new(0.0, h)], turn: true, fold: false };
        // Far taller than the fabric: not laid, and its billions of rows are never made.
        let l = lay(&[rect(100.0, 1e12), rect(100.0, 100.0)], 1000.0, 2.0, 3.0).unwrap();
        assert_eq!(l.placed[0], None);
        assert!(l.placed[1].is_some());
        // Two parts each longer than any grid: too large, without the column count overflowing.
        assert_eq!(lay(&[rect(1e300, 10.0), rect(1e300, 10.0)], 1000.0, 2.0, 3.0), Err(MarkerError::TooLarge));
    }

    #[test]
    fn on_a_plaid_every_part_moves_whole_repeats_and_none_overlap() {
        let at = |x: f64, y: f64, w: f64, h: f64| Part {
            points: vec![Pt::new(x, y), Pt::new(x + w, y), Pt::new(x + w, y + h), Pt::new(x, y + h)],
            turn: true,
            fold: false,
        };
        let parts = vec![at(7.0, 3.0, 120.0, 80.0), at(150.0, 11.0, 90.0, 60.0), at(-40.0, 200.0, 100.0, 100.0)];
        let r = Pt::new(30.0, 25.0);
        let l = lay_matched(&parts, 300.0, 2.0, 5.0, Some(r)).unwrap();
        let laid: Vec<Vec<Pt>> = parts
            .iter()
            .zip(&l.placed)
            .map(|(p, at)| {
                let at = at.unwrap();
                assert!(!at.turned, "a plaid isn't turned");
                let (kx, ky) = (at.offset.x / r.x, at.offset.y / r.y);
                assert!((kx - kx.round()).abs() < 1e-9 && (ky - ky.round()).abs() < 1e-9, "{:?}", at.offset);
                p.points.iter().map(|q| at.apply(*q)).collect()
            })
            .collect();
        for (i, a) in laid.iter().enumerate() {
            assert!(a.iter().all(|q| q.y >= -1e-9 && q.y <= 300.0 + 1e-9 && q.x >= -1e-9), "on the fabric: {a:?}");
            for b in &laid[i + 1..] {
                assert!(!overlaps(a, b), "{a:?} {b:?}");
            }
        }
        assert!(lay_matched(&parts, 300.0, 2.0, 5.0, Some(Pt::new(0.0, 25.0))).is_err());
    }

    #[test]
    fn pieces_laid_by_hand_overlap_when_their_outlines_cross_or_nest() {
        use super::overlaps;
        let sq = |x: f64, y: f64, w: f64| vec![Pt::new(x, y), Pt::new(x + w, y), Pt::new(x + w, y + w), Pt::new(x, y + w)];
        assert!(overlaps(&sq(0.0, 0.0, 10.0), &sq(5.0, 5.0, 10.0)), "crossing");
        assert!(overlaps(&sq(0.0, 0.0, 10.0), &sq(2.0, 2.0, 3.0)), "one inside the other");
        assert!(overlaps(&sq(2.0, 2.0, 3.0), &sq(0.0, 0.0, 10.0)));
        assert!(!overlaps(&sq(0.0, 0.0, 10.0), &sq(10.0, 0.0, 10.0)), "touching edges");
        assert!(!overlaps(&sq(0.0, 0.0, 10.0), &sq(20.0, 0.0, 10.0)), "apart");
        // An L whose notch holds a square that doesn't touch it.
        let l = vec![Pt::new(0.0, 0.0), Pt::new(30.0, 0.0), Pt::new(30.0, 10.0), Pt::new(10.0, 10.0), Pt::new(10.0, 30.0), Pt::new(0.0, 30.0)];
        assert!(!overlaps(&l, &sq(15.0, 15.0, 10.0)));
        assert!(overlaps(&l, &sq(5.0, 5.0, 2.0)));
    }

    use super::*;

    fn rect(w: f64, h: f64) -> Vec<Pt> {
        vec![Pt::new(0.0, 0.0), Pt::new(w, 0.0), Pt::new(w, h), Pt::new(0.0, h)]
    }

    /// Whether two convex or concave polygons' insides overlap, by sampling a fine grid.
    fn overlap(a: &[Pt], b: &[Pt]) -> bool {
        let inside = |pts: &[Pt], q: Pt| {
            let n = pts.len();
            let mut c = false;
            for i in 0..n {
                let (p, r) = (pts[i], pts[(i + 1) % n]);
                if (p.y > q.y) != (r.y > q.y) && q.x < p.x + (r.x - p.x) * (q.y - p.y) / (r.y - p.y) {
                    c = !c;
                }
            }
            c
        };
        let (lo, hi) = a.iter().fold((Pt::new(f64::INFINITY, f64::INFINITY), Pt::new(f64::NEG_INFINITY, f64::NEG_INFINITY)), |(l, h), p| {
            (Pt::new(l.x.min(p.x), l.y.min(p.y)), Pt::new(h.x.max(p.x), h.y.max(p.y)))
        });
        let n = 60;
        (0..=n).any(|i| {
            (0..=n).any(|j| {
                let q = Pt::new(lo.x + (hi.x - lo.x) * i as f64 / n as f64, lo.y + (hi.y - lo.y) * j as f64 / n as f64);
                inside(a, q) && inside(b, q)
            })
        })
    }

    #[test]
    fn rectangles_fill_the_width_then_the_length() {
        // Four 500 × 700 pieces on 1400 wide fabric: two across, two along, 1000 long.
        let parts: Vec<Part> = (0..4).map(|_| Part { points: rect(500.0, 700.0), turn: true, fold: false }).collect();
        let l = lay(&parts, 1400.0, 0.0, 10.0).unwrap();
        assert!(l.placed.iter().all(Option::is_some));
        assert!((l.length - 1000.0).abs() < 1e-9, "{}", l.length);
        // With a 10 gap, the second pair starts 10 further on.
        let l = lay(&parts, 1410.0, 10.0, 5.0).unwrap();
        assert!((l.length - 1010.0).abs() < 5.0 + 1e-9, "{}", l.length);
    }

    #[test]
    fn laid_parts_never_overlap_and_stay_on_the_fabric() {
        // An L, a triangle and rectangles: concave shapes nest into each other.
        let l_shape =
            vec![Pt::new(0.0, 0.0), Pt::new(400.0, 0.0), Pt::new(400.0, 100.0), Pt::new(100.0, 100.0), Pt::new(100.0, 500.0), Pt::new(0.0, 500.0)];
        let tri = vec![Pt::new(0.0, 0.0), Pt::new(300.0, 50.0), Pt::new(20.0, 400.0)];
        let mut parts = vec![Part { points: l_shape.clone(), turn: true, fold: false }, Part { points: l_shape, turn: true, fold: false }];
        parts.extend((0..3).map(|_| Part { points: tri.clone(), turn: true, fold: false }));
        parts.extend((0..3).map(|_| Part { points: rect(250.0, 120.0), turn: false, fold: false }));
        let width = 900.0;
        let l = lay(&parts, width, 4.0, 5.0).unwrap();
        let laid: Vec<Vec<Pt>> = parts.iter().zip(&l.placed).map(|(p, at)| p.points.iter().map(|q| at.unwrap().apply(*q)).collect()).collect();
        for pts in &laid {
            assert!(pts.iter().all(|p| p.y >= -1e-9 && p.y <= width + 1e-9 && p.x >= -1e-9 && p.x <= l.length + 1e-9));
        }
        for i in 0..laid.len() {
            for j in i + 1..laid.len() {
                assert!(!overlap(&laid[i], &laid[j]), "parts {i} and {j} overlap");
            }
        }
        // Closer than laying them in a row.
        assert!(l.length < 1500.0, "{}", l.length);
    }

    #[test]
    fn a_one_way_part_is_never_turned_and_a_too_wide_one_is_left_out() {
        let tri = vec![Pt::new(0.0, 0.0), Pt::new(300.0, 0.0), Pt::new(0.0, 300.0)];
        let parts = vec![
            Part { points: tri.clone(), turn: false, fold: false },
            Part { points: tri, turn: false, fold: false },
            Part { points: rect(100.0, 2000.0), turn: true, fold: false },
        ];
        let l = lay(&parts, 300.0, 0.0, 5.0).unwrap();
        assert!(l.placed[..2].iter().all(|p| p.is_some_and(|p| !p.turned)));
        assert_eq!(l.placed[2], None);
        // Turning lets two triangles make a square.
        let parts: Vec<Part> =
            (0..2).map(|_| Part { points: vec![Pt::new(0.0, 0.0), Pt::new(300.0, 0.0), Pt::new(0.0, 300.0)], turn: true, fold: false }).collect();
        let l = lay(&parts, 300.0, 0.0, 5.0).unwrap();
        assert!(l.length <= 310.0, "{}", l.length);
        assert_eq!(lay(&parts, 0.0, 0.0, 5.0), Err(MarkerError::Width));
        assert_eq!(lay(&parts, 300.0, -1.0, 5.0), Err(MarkerError::Gap));
        assert_eq!(lay(&parts, 300.0, 0.0, 0.0), Err(MarkerError::Step));
    }

    /// Edge cases (#210). A part's count (quantity) is the caller's: it passes that many copies, so
    /// a quantity of 0 is no parts here.
    #[test]
    fn no_parts_make_an_empty_marker_and_unusable_outlines_are_left_out() {
        let l = lay(&[], 1000.0, 2.0, 3.0).unwrap();
        assert!(l.placed.is_empty());
        assert_eq!(l.length, 0.0);
        // No outline, a line and a point are no polygon: not laid, and the others still are.
        let parts = vec![
            Part::default(),
            Part { points: vec![Pt::new(0.0, 0.0), Pt::new(50.0, 0.0)], turn: true, fold: false },
            Part { points: vec![Pt::new(f64::NAN, 0.0), Pt::new(5.0, 0.0), Pt::new(5.0, 5.0)], turn: true, fold: false },
            Part { points: rect(100.0, 100.0), turn: true, fold: false },
        ];
        let l = lay(&parts, 300.0, 0.0, 5.0).unwrap();
        assert_eq!(l.placed[..3], [None, None, None]);
        assert!(l.placed[3].is_some());
        assert!((l.length - 100.0).abs() < 1e-9, "{}", l.length);
        // Only unusable parts: nothing laid, no length.
        let l = lay(&parts[..3], 300.0, 0.0, 5.0).unwrap();
        assert!(l.placed.iter().all(Option::is_none));
        assert_eq!(l.length, 0.0);
    }

    #[test]
    fn a_part_the_width_of_the_fabric_is_laid_and_one_a_row_wider_is_not() {
        let one = |h: f64| lay(&[Part { points: rect(100.0, h), turn: false, fold: false }], 300.0, 0.0, 5.0).unwrap();
        assert!(one(300.0).placed[0].is_some());
        assert_eq!(one(305.0).placed[0], None);
        assert_eq!(one(305.0).length, 0.0);
        // Longer than the fabric is wide is fine: it runs along the length.
        let l = lay(&[Part { points: rect(2000.0, 100.0), turn: false, fold: false }], 300.0, 0.0, 5.0).unwrap();
        assert!(l.placed[0].is_some());
        assert!((l.length - 2000.0).abs() < 1e-6, "{}", l.length);
        // A part too wide is left out and the rest still go on, in order given.
        let parts = vec![
            Part { points: rect(100.0, 500.0), turn: true, fold: false },
            Part { points: rect(100.0, 100.0), turn: true, fold: false },
            Part { points: rect(100.0, 400.0), turn: true, fold: true },
        ];
        let l = lay(&parts, 300.0, 0.0, 5.0).unwrap();
        assert_eq!(l.placed.len(), 3);
        assert_eq!([l.placed[0], l.placed[2]], [None, None]);
        assert!(l.placed[1].is_some());
    }

    #[test]
    fn a_marker_too_big_for_its_limits_is_refused() {
        let part = Part { points: rect(10.0, 10.0), turn: false, fold: false };
        let many: Vec<Part> = (0..=MAX_PARTS).map(|_| part.clone()).collect();
        assert_eq!(lay(&many, 100.0, 0.0, 5.0), Err(MarkerError::TooMany));
        let long = Part { points: vec![Pt::new(0.0, 0.0); MAX_POINTS + 1], turn: false, fold: false };
        assert_eq!(lay(&[long], 100.0, 0.0, 5.0), Err(MarkerError::TooMany));
        for bad in [f64::NAN, f64::INFINITY, -1.0] {
            assert_eq!(lay(&[part.clone()], bad, 0.0, 5.0), Err(MarkerError::Width), "{bad}");
        }
        assert_eq!(lay(&[part.clone()], 100.0, f64::NAN, 5.0), Err(MarkerError::Gap));
        assert_eq!(lay(&[part], 100.0, 0.0, f64::NAN), Err(MarkerError::Step));
    }

    #[test]
    fn a_part_on_the_fold_lies_against_it_and_opens_out_whole() {
        // A half front, centre front (x = 0, from (0, 400) up to (0, 0)) on the fold.
        let half = vec![Pt::new(0.0, 0.0), Pt::new(200.0, 0.0), Pt::new(250.0, 400.0), Pt::new(0.0, 400.0)];
        let whole = opened_out(&half, Pt::new(0.0, 400.0), Pt::new(0.0, 0.0)).unwrap();
        assert_eq!(whole.len(), 6);
        assert!((area(&whole) - 2.0 * area(&half)).abs() < 1e-6);
        assert!(whole.contains(&Pt::new(-250.0, 400.0)) && whole.contains(&Pt::new(-200.0, 0.0)));
        // A cutting line with a point mid-fold too.
        let mid = vec![Pt::new(0.0, 0.0), Pt::new(200.0, 0.0), Pt::new(250.0, 400.0), Pt::new(0.0, 400.0), Pt::new(0.0, 200.0)];
        assert!((area(&opened_out(&mid, Pt::new(0.0, 0.0), Pt::new(0.0, 1.0)).unwrap()) - 2.0 * area(&half)).abs() < 1e-6);
        // Across the line, or away from it: no.
        assert_eq!(opened_out(&half, Pt::new(100.0, 0.0), Pt::new(100.0, 1.0)), None);
        assert_eq!(opened_out(&half, Pt::new(-5.0, 0.0), Pt::new(-5.0, 1.0)), None);
        // On the fold: laid at the top edge, never turned, even with room below.
        let strip = rect(300.0, 100.0);
        let parts = vec![
            Part { points: strip.clone(), turn: true, fold: true },
            Part { points: strip.clone(), turn: true, fold: true },
            Part { points: rect(100.0, 50.0), turn: true, fold: false },
        ];
        let l = lay(&parts, 400.0, 0.0, 5.0).unwrap();
        for at in &l.placed[..2] {
            let at = at.unwrap();
            assert!(!at.turned && strip.iter().map(|p| at.apply(*p).y).fold(f64::INFINITY, f64::min).abs() < 1e-9);
        }
        assert!((l.length - 600.0).abs() < 1e-9, "{}", l.length);
        // A larger part off the fold leaves the fold to the parts on it.
        let parts = vec![Part { points: rect(300.0, 300.0), turn: true, fold: false }, Part { points: strip.clone(), turn: true, fold: true }];
        let l = lay(&parts, 400.0, 0.0, 5.0).unwrap();
        assert!((l.length - 300.0).abs() < 1e-9, "{}", l.length);
    }
}
