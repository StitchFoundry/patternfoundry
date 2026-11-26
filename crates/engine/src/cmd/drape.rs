//! Draping (PF-10.3, PF-10.6, PF-10.10): the document's pieces sewn together (`piece.sew`) and
//! placed round the avatar (`avatar.draw`), skirts from the waist down, bodices from the waist up
//! over the shoulders, sleeves round the arms, trousers down the legs, and let fall onto it as
//! cloth ([`pf_core::cloth`]), for the 3D fit preview.

use super::*;
use pf_core::cloth::{Cloth, Placed, Stiffness, panel};
use pf_core::{Pt, pt_to_mm};
use serde_json::{Value, json};
use vectorcraft_doc::{Document, NodeKind};
use vectorcraft_geom::kurbo::ParamCurveArclen;
use vectorcraft_geom::{Point, SubPath};

/// The default distance between particles, mm: finer needs many more corrections a step to hold
/// the cloth's length (a skirt at 10 mm and the same 40 slid off), and takes longer.
const STEP_MM: f64 = 20.0;
/// The default number of steps, a sixtieth of a second each: four seconds of falling.
const STEPS: usize = 240;

pub fn specs() -> Vec<CommandSpec> {
    vec![cmd!(
        "drape.make",
        "Drape on Avatar",
        [],
        None,
        "{ids? (the pieces; default every piece sewn to another, piece.sew), precision?: mm (20, the distance between the cloth's particles, 15 to 50), steps? (240: four seconds, 1 to 2000), path? (a file to write the draped cloth to: .glb, a binary glTF 2.0 file of the avatar and the cloth in the drape view's colours, metres, y up, the front toward +z, for 3D software and game engines; else an OBJ file of the cloth), colour?: fabric|fit (glb: the cloth in the document's fabric photos, fabric.set, a textured part for each, the photo repeating, the default where there are any; fit: in the drape view's colours), format?: obj|glb (default from path's extension; without a path the file comes back as data (obj) or dataBase64 (glb)), draw?: true (the drape view: the avatar and the draped cloth as pictures seen from the front and the left side, at true scale, the cloth coloured blue where it is loose, green, and red where it is 10% or more larger than cut, on the layer \"Drape\", drawn again it replaces it; false: none), origin?: [x, y] (document points: the front view's top-left; default right of the art), background?: false (in the app: the drape runs off the UI thread, its progress shown with Cancel, and is drawn when it is done; Pattern › Drape on Avatar does this)} sew the pieces up round the avatar (see avatar.draw) and let them fall onto it as light woven cloth. Each piece goes where it belongs: a bodice (a bodice block's part, or named so) from the waist up over the shoulders, a sleeve (a sleeve block's, or named so) round the arm of the armhole it is sewn into, trousers (a trouser block's, or named trousers, pants or shorts; a front and a back, each a mirror-image pair, the back named so) down the legs, each leg's front and back round it below the crotch and the four round the hips above, their centre seams (each sewn to its mirror, piece.sew) at the middle of the front and back, anything else (a skirt) from the waist down; the skirt's and the bodice's pieces round the body in the order they are sewn, a front first, in the middle of the front. Pieces are taken as drawn upright, as patterns are, and as one side of the body: one cut on the fold is opened out whole, one cut as a mirror-image pair (cut 2, mirror) is both sides, its seams sewn on each; others are taken as drawn. Darts (two legs about as long meeting at a sharp point, the mouth open) are sewn shut, and seams are sewn with their darts closed. The first third of the time the garment is sewn up without gravity, its seams pulling harder and harder, then it falls into place; with sleeves the rest goes on first (half the time), then the sleeves are sewn in with the rest held (a sixth, without gravity), then everything settles → {pieces, darts (sewn shut), particles, top (mm above the floor: the highest the cloth hangs), hem (mm: its lowest), waist (mm: the avatar's), worstStretch (%), widestSeam (mm), path?, data?, dataBase64?, layer?, front?, side?, fabric? (with fabric photos, fabric.set: the front view again in them, 1 mm a pixel, right of the side view), tightest (%: the most a triangle of cloth is larger than cut), fit (loose|snug|toottight|openseam: a stretch over 30% is toottight, a seam gap over 20 mm openseam), fitMessage (the fit in one plain line)}",
        has_doc,
        make
    )]
}

/// The closed seam line of piece `id`.
fn seam_line(doc: &Document, id: NodeId) -> Option<SubPath> {
    let seam = super::piece::seam_of(doc, id)?;
    match &seam.kind {
        NodeKind::Path { path, .. } => path.subpaths.iter().find(|sp| sp.closed).cloned(),
        _ => None,
    }
}

/// `n` points evenly along `sp` (by length) from anchor `from` forward to anchor `to`, in mm,
/// leaving out the edges `skip` (a dart's legs: a seam is sewn with its darts closed).
fn run_points(sp: &SubPath, from: usize, to: usize, n: usize, skip: &[usize]) -> Vec<Pt> {
    let edges = sp.segment_count();
    if edges == 0 || n < 2 {
        return Vec::new();
    }
    let mut segs = Vec::new();
    let mut e = from % edges;
    for _ in 0..edges {
        if !skip.contains(&e) {
            segs.push((e, sp.segment(e).arclen(1e-6)));
        }
        e = (e + 1) % edges;
        if e == to % edges {
            break;
        }
    }
    if segs.is_empty() {
        return Vec::new();
    }
    let total: f64 = segs.iter().map(|s| s.1).sum();
    (0..n)
        .filter_map(|k| {
            let mut d = total * k as f64 / (n - 1) as f64;
            for &(e, len) in &segs {
                if d <= len + 1e-9 || e == segs[segs.len() - 1].0 {
                    let at = if len > 1e-9 { (d / len).clamp(0.0, 1.0) } else { 0.0 };
                    return super::piece::point_along(sp, e, at).map(|q: Point| Pt::new(pt_to_mm(q.x), pt_to_mm(q.y)));
                }
                d -= len;
            }
            None
        })
        .collect()
}

/// Where polygon `pts` crosses the level `y`: its leftmost and rightmost crossings.
fn span(pts: &[Pt], y: f64) -> Option<(f64, f64)> {
    let n = pts.len();
    let mut out: Option<(f64, f64)> = None;
    for i in 0..n {
        let (a, b) = (pts[i], pts[(i + 1) % n]);
        let (lo, hi) = if a.y <= b.y { (a, b) } else { (b, a) };
        if y < lo.y || y > hi.y {
            continue;
        }
        let x = if (hi.y - lo.y).abs() < 1e-12 { lo.x.min(hi.x) } else { lo.x + (y - lo.y) / (hi.y - lo.y) * (hi.x - lo.x) };
        let xs = if (hi.y - lo.y).abs() < 1e-12 { [lo.x.min(hi.x), lo.x.max(hi.x)] } else { [x, x] };
        for x in xs {
            out = Some(out.map_or((x, x), |(l, r)| (l.min(x), r.max(x))));
        }
    }
    out
}

/// Where a copy's points on the pattern go round the body.
type Placing = Box<dyn Fn(Pt) -> [f64; 3]>;

/// Where on the body a piece goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Role {
    /// From the waist down (a skirt): its top at the waist.
    Lower,
    /// From the waist up (a bodice): its bottom at the waist, over the shoulders.
    Upper,
    /// Round an arm (a sleeve): its top at the shoulder point.
    Arm,
    /// Round a leg (trousers): its top at the waist, round the hips above the crotch.
    Leg,
}

/// A piece's role: what its block is (a sleeve, a bodice), else what its name says, else a skirt's.
fn role_of(doc: &Document, id: NodeId, name: &str) -> Role {
    use pf_core::block::BlockKind;
    let kind = super::piece::seam_of(doc, id).and_then(|n| n.extras().block.as_ref().map(|b| b.kind));
    let name = name.to_lowercase();
    match kind {
        Some(BlockKind::Sleeve | BlockKind::MensShirtSleeve) => Role::Arm,
        Some(BlockKind::Bodice | BlockKind::MensShirt) => Role::Upper,
        Some(BlockKind::Trousers | BlockKind::MensTrousers) => Role::Leg,
        Some(_) => Role::Lower,
        None if name.contains("sleeve") => Role::Arm,
        None if name.contains("trouser") || name.contains("pant") || name.contains("short") => Role::Leg,
        None if name.contains("bodice") || name.contains("top") || name.contains("yoke") => Role::Upper,
        None => Role::Lower,
    }
}

/// One copy of a piece in the garment: the piece opened out across its fold (whole, both sides of
/// the body), or one of a mirror-image pair (`mirrored`: the body's other side), or the piece as
/// drawn.
struct Copy {
    piece: usize,
    mirrored: bool,
    panel: pf_core::cloth::Panel,
    outline: Vec<Pt>,
    /// Its bounds on the pattern: left, right, top, bottom (mm, y down).
    bounds: (f64, f64, f64, f64),
    /// Its grainline's direction on the pattern (a unit vector, down the piece).
    grain: Pt,
}

/// A piece of the garment: its seam line (mm on the pattern) and how it is cut.
struct Part {
    id: NodeId,
    name: String,
    sp: SubPath,
    role: Role,
    /// The fold line, for a piece cut on the fold.
    fold: Option<(Pt, Pt)>,
    /// Its middle across (for mirroring a pair).
    middle: f64,
    /// Its darts (see [`darts`]) and the edges their legs are.
    darts: Vec<((usize, usize), (usize, usize))>,
    legs: Vec<usize>,
    /// Its copies for the body's two sides (the same copy for both when opened out on the fold).
    sides: [Option<usize>; 2],
    /// The fabric it is cut from (`Piece::fabric`).
    fabric: String,
}

/// `p` reflected across the line through `a` and `b`.
fn reflect(p: Pt, a: Pt, b: Pt) -> Pt {
    let Some(d) = (b - a).unit() else { return p };
    let v = p - a;
    let along = d * v.dot(d);
    a + along * 2.0 - v
}

/// Whether `q` is inside polygon `pts` (even-odd).
fn inside(pts: &[Pt], q: Pt) -> bool {
    let mut c = false;
    let n = pts.len();
    for i in 0..n {
        let (a, b) = (pts[i], pts[(i + 1) % n]);
        if (a.y > q.y) != (b.y > q.y) && q.x < a.x + (q.y - a.y) / (b.y - a.y) * (b.x - a.x) {
            c = !c;
        }
    }
    c
}

/// The darts of seam line `sp`: anchors where the line goes in to a point and back out along two
/// legs about as long, the mouth between them open (outside the piece) → (the leg before, the leg
/// after: each its anchors, mouth to point).
fn darts(sp: &SubPath, outline: &[Pt]) -> Vec<((usize, usize), (usize, usize))> {
    let n = sp.segment_count();
    if n < 4 {
        return Vec::new();
    }
    let at = |i: usize| {
        let q = sp.segment(i % n).p0;
        Pt::new(pt_to_mm(q.x), pt_to_mm(q.y))
    };
    let mut out = Vec::new();
    for k in 0..n {
        let (a, apex, b) = (at(k + n - 1), at(k), at(k + 1));
        let (u, v) = (a - apex, b - apex);
        let (lu, lv) = (u.len(), v.len());
        if lu < 15.0 || lv < 15.0 || (lu - lv).abs() > 0.2 * lu.max(lv) {
            continue;
        }
        let cos = u.dot(v) / (lu * lv);
        let mouth = (a + b) * 0.5;
        // A dart's legs meet at a sharp point; its mouth (between the legs' outer ends) is not
        // cloth.
        if cos > 45f64.to_radians().cos() && !inside(outline, mouth) && inside(outline, apex + (mouth - apex) * 0.02) == inside(outline, mouth) {
            out.push((((k + n - 1) % n, k), ((k + 1) % n, k)));
        }
    }
    out
}

/// The particles of `panel` along the run `pts` (within a third of a step of it) → (how far along
/// it, 0 to 1; the particle), in order along it.
fn along(panel: &pf_core::cloth::Panel, pts: &[Pt], step: f64) -> Vec<(f64, u32)> {
    let mut cum = vec![0.0];
    for w in pts.windows(2) {
        cum.push(cum[cum.len() - 1] + (w[1] - w[0]).len());
    }
    let total = cum[cum.len() - 1].max(1e-9);
    let mut out: Vec<(f64, u32)> = Vec::new();
    for (i, q) in panel.points.iter().enumerate() {
        let mut best = (f64::INFINITY, 0.0);
        for (k, w) in pts.windows(2).enumerate() {
            let d = w[1] - w[0];
            let l2 = d.dot(d);
            let t = if l2 > 1e-12 { ((*q - w[0]).dot(d) / l2).clamp(0.0, 1.0) } else { 0.0 };
            let dist = (*q - (w[0] + d * t)).len();
            if dist < best.0 {
                best = (dist, (cum[k] + t * l2.sqrt()) / total);
            }
        }
        if best.0 < step / 3.0 {
            out.push((best.1, i as u32));
        }
    }
    out.sort_by(|a, b| a.0.total_cmp(&b.0));
    out
}

/// A drape set up and ready to run: the cloth placed round the avatar with its seams and darts
/// sewn, and how many steps to run it for.
pub struct Setup {
    pub cloth: Cloth,
    pub avatar: pf_core::avatar::Avatar,
    pub steps: usize,
    pub iterations: usize,
    pub waist: f64,
    pub pieces: usize,
    /// The darts sewn shut (each copy's).
    pub darts: usize,
    /// The sleeves' particles, set in after the rest is on ([`Setup::run`]).
    pub sleeves: Vec<u32>,
    /// The stitches setting the sleeves in (sleeve to armhole), by their index in `cloth.seams`.
    pub set_in: Vec<usize>,
    /// Each panel's (copy's) fabric and grainline (its direction on the pattern, down the piece),
    /// for the view in the fabric's photo (PF-10.16).
    pub grains: Vec<(String, Pt)>,
}

impl Setup {
    /// Let the cloth fall for its steps (four substeps a sixtieth of a second each), telling
    /// `progress` the share done after each; it stops (→ false) when `progress` says to.
    pub fn run(&mut self, progress: &mut dyn FnMut(f64) -> bool) -> bool {
        // Put on as a dresser would: with sleeves, the rest first (the sleeves held still, not yet
        // set in), then the sleeves sewn in with the rest held; then everything let go.
        let staged = !self.sleeves.is_empty() && !self.set_in.is_empty();
        let body_part = if staged { self.steps / 2 } else { self.steps };
        let setting = if staged { self.steps / 6 } else { 0 };
        if staged {
            for &i in &self.sleeves {
                self.cloth.hold(i, true);
            }
            for &k in &self.set_in {
                self.cloth.seam_on[k] = false;
            }
        }
        for k in 0..self.steps {
            if staged && k == body_part {
                self.set_sleeves_in();
            }
            if staged && k == body_part + setting {
                self.let_go();
            }
            // Each part sewn up round the body first: no gravity, the seams pulling harder and
            // harder (a hundredth as hard at first) so the pieces come together without yanking;
            // then it falls into place and settles.
            let (start, length) = if staged && k >= body_part { (body_part, setting.max(1)) } else { (0, (body_part / 3).max(1)) };
            let at = (k - start).min(length);
            self.cloth.gravity = if at < length || (staged && k < body_part + setting && k >= body_part) { 0.0 } else { 1.0 };
            self.cloth.seam_pull = 0.5 * (0.01f64).powf(1.0 - at as f64 / length as f64);
            // Slippery while it is sewn up, so the pieces slide into place; gripping the body once
            // it hangs.
            self.cloth.grip = if self.cloth.gravity == 0.0 { 0.0 } else { pf_core::cloth::GRIP };
            for _ in 0..4 {
                self.cloth.step(1.0 / 240.0, self.iterations, &self.avatar);
            }
            if !progress((k + 1) as f64 / self.steps as f64) {
                return false;
            }
        }
        true
    }

    /// The sleeves set in: the rest held where it lies, the sleeves let go and sewn to it.
    fn set_sleeves_in(&mut self) {
        let n = self.cloth.positions.len() as u32;
        let sleeve: std::collections::HashSet<u32> = self.sleeves.iter().copied().collect();
        for i in (0..n).filter(|i| !sleeve.contains(i)) {
            self.cloth.hold(i, true);
        }
        for &i in &self.sleeves {
            self.cloth.hold(i, false);
        }
        // The set-in seams pull from the start of this stage, ramped up from a hundredth as hard
        // (moving each crown stitch straight onto its armhole stitch instead pulled one sleeve
        // through its arm).
        for &k in &self.set_in {
            self.cloth.seam_on[k] = true;
        }
    }

    /// Everything let go and every seam pulling.
    fn let_go(&mut self) {
        for i in 0..self.cloth.positions.len() as u32 {
            self.cloth.hold(i, false);
        }
        for on in &mut self.cloth.seam_on {
            *on = true;
        }
    }
}

/// Trouser pieces (role [`Role::Leg`]) placed: each leg's front and back round its leg below the
/// crotch, the four together round the hips above, blended between the hip and the crotch; each
/// piece's top at the waist (the back's rise above it), its centre seam (`centre`: each copy's,
/// its mean across on the pattern) at the middle of the front or back, the inseam inside the leg.
/// A leg needs a front and a back (by name: "back", else a front); left unplaced otherwise.
fn legs(place: &mut [Option<Placing>], copies: &[Copy], parts: &[Part], centre: &[(usize, f64)], avatar: &pf_core::avatar::Avatar, waist: f64) {
    use std::f64::consts::PI;
    let back = |c: usize| parts[copies[c].piece].name.to_lowercase().contains("back");
    let leg_copies: Vec<usize> = (0..copies.len()).filter(|c| parts[copies[*c].piece].role == Role::Leg).collect();
    // The centre seam's side of each copy: from its seam, else as the trouser block draws them
    // (the front's centre on its left, the back's on its right; mirrored, the other way).
    let centre_left = |c: usize| {
        let (l, r, _, _) = copies[c].bounds;
        centre.iter().find(|(k, _)| *k == c).map_or(back(c) == copies[c].mirrored, |(_, x)| *x < (l + r) / 2.0)
    };
    // Which leg (+1 the body's +x side): seen from the front a front's centre is toward -x, seen
    // from behind a back's toward the viewer's right, -x too.
    let side = |c: usize| if centre_left(c) != back(c) { 1.0 } else { -1.0 };
    let find = |sd: f64, b: bool| leg_copies.iter().copied().find(|c| side(*c) == sd && back(*c) == b);
    let (Some(f0), Some(b0), Some(f1), Some(b1)) = (find(1.0, false), find(1.0, true), find(-1.0, false), find(-1.0, true)) else {
        return;
    };
    // Heights from the hems, all at the ankle line together: the front's top at the waist.
    let length = copies[f0].bounds.3 - copies[f0].bounds.2;
    let foot = waist - length;
    let crotch = avatar.ring("crotch").map_or(waist - 250.0, |r| r.z);
    let hip = avatar.ring("hip").map_or(waist - 200.0, |r| r.z).max(crotch + 1.0);
    let widths: std::sync::Arc<Vec<(Vec<Pt>, f64)>> = std::sync::Arc::new(copies.iter().map(|c| (c.outline.clone(), c.bounds.3)).collect());
    let x_of = |rings: &[pf_core::avatar::Ring], z: f64| {
        rings
            .windows(2)
            .find(|w| z <= w[0].z && z >= w[1].z)
            .map(|w| w[0].x + (w[1].x - w[0].x) * (w[0].z - z) / (w[0].z - w[1].z))
            .unwrap_or_else(|| if z > rings[0].z { rings[0].x } else { rings[rings.len() - 1].x })
    };
    let legs_x: [Vec<pf_core::avatar::Ring>; 2] = [avatar.legs[0].clone(), avatar.legs[1].clone()];
    for &(c, sd, b) in &[(f0, 1.0, false), (b0, 1.0, true), (f1, -1.0, false), (b1, -1.0, true)] {
        let (mine, other) = if sd > 0.0 { ((f0, b0), (f1, b1)) } else { ((f1, b1), (f0, b0)) };
        let widths = widths.clone();
        let left = centre_left(c);
        let bottom = copies[c].bounds.3;
        let rings = if sd > 0.0 { legs_x[0].clone() } else { legs_x[1].clone() };
        let cx = move |z: f64| x_of(&rings, z).abs();
        place[c] = Some(Box::new(move |q: Pt| {
            let h = bottom - q.y;
            let z = foot + h;
            let w = |k: usize| span(&widths[k].0, widths[k].1 - h).map_or(0.0, |(a, b)| b - a);
            let (wf, wb) = (w(mine.0), w(mine.1));
            let (l, r) = span(&widths[c].0, q.y).unwrap_or((q.x, q.x + 1.0));
            // How far from the centre seam across the piece, 0 to 1 (at the side seam).
            let s = (if left { q.x - l } else { r - q.x } / (r - l).max(1e-9)).clamp(0.0, 1.0);
            // Round the hips: the front from the middle of the front, the back on to the middle
            // of the back.
            let all = (wf + wb + w(other.0) + w(other.1)).max(1.0);
            let t = if b { 2.0 * PI * (wf + (1.0 - s) * wb) / all } else { 2.0 * PI * s * wf / all };
            let ring = [sd * all / (2.0 * PI) * t.sin(), all / (2.0 * PI) * t.cos()];
            // Round the leg: the front across the front, the inseam inside.
            let both = (wf + wb).max(1.0);
            let phi = if b { PI * wf / both + (1.0 - s) * 2.0 * PI * wb / both } else { -PI * wf / both + s * 2.0 * PI * wf / both };
            let rl = both / (2.0 * PI);
            let tube = [sd * (cx(z) + rl * phi.sin()), rl * phi.cos()];
            let a = ((z - crotch) / (hip - crotch)).clamp(0.0, 1.0);
            [tube[0] + (ring[0] - tube[0]) * a, tube[1] + (ring[1] - tube[1]) * a, z]
        }));
    }
}

/// Set up the drape of `p` (see `drape.make`).
pub fn setup(s: &Session, p: &Value) -> Result<Setup> {
    const C: &str = "drape.make";
    let step = match p.get("precision") {
        None | Some(Value::Null) => STEP_MM,
        Some(v) => super::piece::mm_param(v).filter(|m| (15.0..=50.0).contains(m)).ok_or_else(|| bad(C, "precision: 15 to 50 mm"))?,
    };
    let steps = match p.get("steps") {
        None | Some(Value::Null) => STEPS,
        Some(v) => v.as_u64().filter(|n| (1..=2000).contains(n)).ok_or_else(|| bad(C, "steps: 1 to 2000"))? as usize,
    };
    let doc = &s.doc()?.doc;
    let avatar = super::avatar::avatar_of(doc, C)?;
    let runs = super::piece::sewn_runs(doc);
    // The pieces: those named, else every piece with a sewn seam, in the order they are sewn.
    let named: Option<Vec<NodeId>> = if p.get("ids").is_some() { Some(super::piece::target_pieces(s, p, C)?) } else { None };
    let mut ids: Vec<NodeId> = Vec::new();
    for r in &runs {
        if !ids.contains(&r.1) && named.as_ref().is_none_or(|n| n.contains(&r.1)) {
            ids.push(r.1);
        }
    }
    if ids.is_empty() {
        return Err(bad(C, "no pieces are sewn together to drape (piece.sew sews their seams)"));
    }
    let no_line = || bad(C, "a sewn piece has no closed seam line");
    // Each piece and its copies: opened out across its fold, or a mirror-image pair, or as drawn.
    let mut parts: Vec<Part> = Vec::new();
    let mut copies: Vec<Copy> = Vec::new();
    for &id in &ids {
        let sp = seam_line(doc, id).ok_or_else(no_line)?;
        let (outline, _) = super::piece::outline_of(&vectorcraft_geom::PathData::single(sp.clone())).ok_or_else(no_line)?;
        let mm: Vec<Pt> = outline.points.iter().map(|q| Pt::new(pt_to_mm(q.x), pt_to_mm(q.y))).collect();
        let piece = doc.node(id).and_then(|n| super::piece::piece_on(n).cloned()).ok_or_else(no_line)?;
        let anchor = |i: usize| {
            let q = sp.segment(i % sp.segment_count()).p0;
            Pt::new(pt_to_mm(q.x), pt_to_mm(q.y))
        };
        let fold = piece.fold.map(|e| (anchor(e), anchor(e + 1)));
        let (x0, x1) = mm.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), q| (a.min(q.x), b.max(q.x)));
        let k = parts.len();
        // Down the grainline (vertical without one); a mirrored copy's the other way across.
        let grain = piece
            .grainline
            .map(|g| Pt::new(g.to.x - g.from.x, g.to.y - g.from.y))
            .and_then(|d| {
                let len = d.x.hypot(d.y);
                (len > 1e-9).then(|| if d.y < 0.0 { Pt::new(-d.x / len, -d.y / len) } else { Pt::new(d.x / len, d.y / len) })
            })
            .unwrap_or(Pt::new(0.0, 1.0));
        let mut add = |outline: Vec<Pt>, mirrored: bool| -> Result<usize> {
            let panel = panel(&outline, step).map_err(|e| bad(C, e.to_string()))?;
            let b = outline.iter().fold((f64::INFINITY, f64::NEG_INFINITY, f64::INFINITY, f64::NEG_INFINITY), |(l, r, t, bo), q| {
                (l.min(q.x), r.max(q.x), t.min(q.y), bo.max(q.y))
            });
            copies.push(Copy { piece: k, mirrored, panel, outline, bounds: b, grain: if mirrored { Pt::new(-grain.x, grain.y) } else { grain } });
            Ok(copies.len() - 1)
        };
        let sides = match fold.and_then(|(a, b)| pf_core::marker::opened_out(&mm, a, b)) {
            Some(whole) => {
                let c = add(whole, false)?;
                [Some(c), Some(c)]
            }
            None if piece.cut >= 2 && piece.mirror => {
                let flipped: Vec<Pt> = mm.iter().rev().map(|q| Pt::new(x0 + x1 - q.x, q.y)).collect();
                [Some(add(mm.clone(), false)?), Some(add(flipped, true)?)]
            }
            None => [Some(add(mm.clone(), false)?), None],
        };
        let role = role_of(doc, id, &piece.name);
        let found = darts(&sp, &mm);
        let legs = found.iter().flat_map(|((f, k), _)| [*f, *k]).collect();
        parts.push(Part {
            id,
            name: piece.name.clone(),
            sp,
            role,
            fold: if sides[0] == sides[1] { fold } else { None },
            middle: (x0 + x1) / 2.0,
            darts: found,
            legs,
            sides,
            fabric: piece.fabric.clone(),
        });
    }
    // A run's points on copy `c` of its piece, for the body's side `side` (0 as drawn, 1 the other).
    let on_copy = |part: &Part, side: usize, pts: &[Pt]| -> Option<(usize, Vec<Pt>)> {
        let c = part.sides[side]?;
        let pts = match (side, part.fold, copies[c].mirrored) {
            (1, Some((a, b)), _) => pts.iter().map(|q| reflect(*q, a, b)).collect(),
            (_, _, true) => pts.iter().map(|q| Pt::new(2.0 * part.middle - q.x, q.y)).collect(),
            _ => pts.to_vec(),
        };
        Some((c, pts))
    };
    // The seams between copies: each sewn seam on both sides of the body where its pieces have
    // copies there, and each dart sewn shut (its legs from the mouth to the point).
    struct Seam {
        a: usize,
        b: usize,
        pa: Vec<Pt>,
        pb: Vec<Pt>,
        /// Both runs go the same way (a dart's legs, mouth to point); else matched by height.
        ordered: bool,
    }
    let mut seams: Vec<Seam> = Vec::new();
    let part_of = |id: NodeId| parts.iter().position(|q| q.id == id);
    let labels: std::collections::BTreeSet<&String> = runs.iter().map(|r| &r.0).collect();
    for label in labels {
        let pair: Vec<&(String, NodeId, String, usize, usize)> = runs.iter().filter(|r| &r.0 == label).collect();
        let [ra, rb] = pair[..] else { continue };
        let (Some(ia), Some(ib)) = (part_of(ra.1), part_of(rb.1)) else { continue };
        let pa = run_points(&parts[ia].sp, ra.3, ra.4, 64, &parts[ia].legs);
        let pb = run_points(&parts[ib].sp, rb.3, rb.4, 64, &parts[ib].legs);
        // A centre seam: a run sewn to itself, on a mirror-image pair: the copy as drawn sewn to
        // its mirror.
        if ia == ib && (ra.3, ra.4) == (rb.3, rb.4) {
            if let (Some((a, pa)), Some((b, pb))) = (on_copy(&parts[ia], 0, &pa), on_copy(&parts[ia], 1, &pb))
                && a != b
            {
                seams.push(Seam { a, b, pa, pb, ordered: false });
            }
            continue;
        }
        for side in 0..2 {
            if let (Some((a, pa)), Some((b, pb))) = (on_copy(&parts[ia], side, &pa), on_copy(&parts[ib], side, &pb)) {
                // Both sides on one copy opened out: the second is its mirror, a seam of its own.
                seams.push(Seam { a, b, pa, pb, ordered: false });
            }
        }
    }
    for part in &parts {
        for &((f1, t1), (f2, t2)) in &part.darts {
            let leg1 = run_points(&part.sp, f1, t1, 32, &[]);
            let mut leg2 = run_points(&part.sp, t2, f2, 32, &[]);
            leg2.reverse();
            for side in 0..2 {
                if let (Some((a, pa)), Some((_, pb))) = (on_copy(part, side, &leg1), on_copy(part, side, &leg2)) {
                    seams.push(Seam { a, b: a, pa, pb, ordered: true });
                }
            }
        }
    }
    let waist = avatar.ring("waist").map_or(1000.0, |r| r.z);
    let shoulder = avatar.ring("shoulder").map_or(waist + 350.0, |r| r.z);
    let neck = avatar.neck_height;
    // Round the torso: the skirt's copies in one ring and the bodice's in another, each in the
    // order they are sewn round the body (a front first, in the middle of the front), each copy
    // turned so its seam to the next is on the far side.
    let role = |c: usize| parts[copies[c].piece].role;
    let vertical = |s: &Seam| s.pa.iter().map(|q| q.y).fold(f64::NEG_INFINITY, f64::max) - s.pa.iter().map(|q| q.y).fold(f64::INFINITY, f64::min);
    let mut place: Vec<Option<Placing>> = (0..copies.len()).map(|_| None).collect();
    for group in [Role::Lower, Role::Upper] {
        let members: Vec<usize> = (0..copies.len()).filter(|c| role(*c) == group).collect();
        if members.is_empty() {
            continue;
        }
        let first = members.iter().copied().find(|c| parts[copies[*c].piece].name.to_lowercase().contains("front")).unwrap_or(members[0]);
        let mut order = vec![first];
        while order.len() < members.len() {
            let last = order[order.len() - 1];
            let next = seams
                .iter()
                .filter(|s| s.a != s.b && (s.a == last || s.b == last))
                .map(|s| if s.a == last { s.b } else { s.a })
                .find(|c| members.contains(c) && !order.contains(c))
                .or_else(|| members.iter().copied().find(|c| !order.contains(c)));
            match next {
                Some(c) => order.push(c),
                None => break,
            }
        }
        // Each copy turned so the seam to the next copy round is on its far side: going round, each
        // join takes a seam between the two not taken yet (a front and a back are sewn at both
        // sides: the first join is one side seam, the way back the other).
        let mut taken: Vec<usize> = Vec::new();
        let flips: Vec<bool> = (0..order.len())
            .map(|k| {
                let (me, next) = (order[k], order[(k + 1) % order.len()]);
                let joint = seams
                    .iter()
                    .enumerate()
                    .filter(|(i, s)| s.a != s.b && !taken.contains(i) && ((s.a == me && s.b == next) || (s.b == me && s.a == next)))
                    .max_by(|x, y| vertical(x.1).total_cmp(&vertical(y.1)));
                match joint {
                    Some((i, s)) if order.len() > 1 => {
                        taken.push(i);
                        let pts = if s.a == me { &s.pa } else { &s.pb };
                        let mx = pts.iter().map(|q| q.x).sum::<f64>() / pts.len().max(1) as f64;
                        let (l, r, _, _) = copies[me].bounds;
                        mx < (l + r) / 2.0
                    }
                    _ => false,
                }
            })
            .collect();
        let upward = group == Role::Upper;
        // Each row of the cloth (by its height from the waist on its piece) wrapped round a circle
        // as long round as the copies are wide together at that height, so the sewn sides start
        // together; the first copy's middle at the front.
        let row_of = move |c: &Copy, y: f64| if upward { ((c.bounds.3 - y) / step).round() as i64 } else { ((y - c.bounds.2) / step).round() as i64 };
        let extents: Vec<std::collections::BTreeMap<i64, (f64, f64)>> = order
            .iter()
            .map(|&c| {
                let mut e = std::collections::BTreeMap::new();
                for q in &copies[c].panel.points {
                    e.entry(row_of(&copies[c], q.y)).or_insert_with(|| span(&copies[c].outline, q.y).unwrap_or((q.x, q.x)));
                }
                e
            })
            .collect();
        let extents = std::sync::Arc::new(extents);
        for (k, &c) in order.iter().enumerate() {
            let (extents, flip, bounds) = (extents.clone(), flips[k], copies[c].bounds);
            let count = order.len();
            let width = move |e: &[std::collections::BTreeMap<i64, (f64, f64)>], j: usize, r: i64| e[j].get(&r).map_or(0.0, |(a, b)| b - a);
            place[c] = Some(Box::new(move |q: Pt| {
                let r = if upward { ((bounds.3 - q.y) / step).round() as i64 } else { ((q.y - bounds.2) / step).round() as i64 };
                let total: f64 = (0..count).map(|j| width(&extents, j, r)).sum::<f64>().max(step);
                let before: f64 = (0..k).map(|j| width(&extents, j, r)).sum::<f64>() - width(&extents, 0, r) / 2.0;
                let (l, rr) = extents[k].get(&r).copied().unwrap_or((q.x, q.x));
                let u = if flip { rr - q.x } else { q.x - l };
                let t = 2.0 * std::f64::consts::PI * (before + u) / total;
                let radius = total / (2.0 * std::f64::consts::PI);
                // A top from the waist up, or (one longer than the body is from the neck to the
                // waist, a shirt) from the base of the neck down, whichever is lower.
                let z = if upward { (waist + (bounds.3 - q.y)).min(neck - (q.y - bounds.2)) } else { waist - (q.y - bounds.2) };
                [radius * t.sin(), radius * t.cos(), z]
            }));
        }
    }
    // Trousers: where each leg copy's centre seam is (its mean across, on the pattern).
    let centre: Vec<(usize, f64)> = seams
        .iter()
        .filter(|s| s.a != s.b && copies[s.a].piece == copies[s.b].piece && role(s.a) == Role::Leg)
        .flat_map(|s| {
            let mean = |pts: &[Pt]| pts.iter().map(|q| q.x).sum::<f64>() / pts.len().max(1) as f64;
            [(s.a, mean(&s.pa)), (s.b, mean(&s.pb))]
        })
        .collect();
    legs(&mut place, &copies, &parts, &centre, &avatar, waist);
    // The sleeves, each round the arm on the side of the armhole it is sewn into (else the body's
    // left for the copy as drawn, its right for the other): its top at the shoulder point, the
    // middle of the crown outside the arm.
    let sleeves: Vec<usize> = (0..copies.len()).filter(|c| role(*c) == Role::Arm).collect();
    for (n, &c) in sleeves.iter().enumerate() {
        // The armhole it is sewn into: where it is round the body (x across, y to the front), and
        // which half of the sleeve is sewn to it (+1 the half right of the sleeve's middle).
        let (l, r, top, _) = copies[c].bounds;
        let partner = seams.iter().filter(|s| s.a != s.b && (s.a == c || s.b == c)).find_map(|s| {
            let (other, theirs, mine) = if s.a == c { (s.b, &s.pb, &s.pa) } else { (s.a, &s.pa, &s.pb) };
            let f = place[other].as_ref()?;
            let n = theirs.len().max(1) as f64;
            let (x, y) = theirs.iter().map(|q| f(*q)).fold((0.0, 0.0), |(x, y), p| (x + p[0] / n, y + p[1] / n));
            let half = if mine.iter().map(|q| q.x).sum::<f64>() / mine.len().max(1) as f64 > (l + r) / 2.0 { 1.0 } else { -1.0 };
            Some((x, y, half))
        });
        let partner_x = partner.map(|p| p.0);
        let arm = match partner_x {
            Some(x) if x < 0.0 => 1,
            Some(_) => 0,
            None => usize::from(copies[c].mirrored || n % 2 == 1),
        };
        let rings = avatar.arms[arm].clone();
        let side = if arm == 0 { 1.0 } else { -1.0 };
        // The half sewn into the front of the armhole to the front.
        let toward = partner.map_or(1.0, |(_, y, half)| if y * half < 0.0 { -1.0 } else { 1.0 });
        // The armpit: the first height down the sleeve where it is (nearly) as wide as it is.
        let under = {
            let o = &copies[c].outline;
            let w = r - l;
            let mut y = top;
            while y < copies[c].bounds.3 && span(o, y).is_none_or(|(a, b)| b - a < 0.95 * w) {
                y += 1.0;
            }
            y
        };
        place[c] = Some(Box::new(move |q: Pt| {
            let z = shoulder - (q.y - top);
            let x_at = rings
                .windows(2)
                .find(|w| z <= w[0].z && z >= w[1].z)
                .map(|w| {
                    let t = (w[0].z - z) / (w[0].z - w[1].z);
                    w[0].x + (w[1].x - w[0].x) * t
                })
                .unwrap_or_else(|| if z > rings[0].z { rings[0].x } else { rings[rings.len() - 1].x });
            // Round a tube as wide as the sleeve: the crown (narrower) only part of the way round,
            // its seam looping from the top of the arm round the front and back to the armpit, as
            // a set-in sleeve's does.
            let w = (r - l).max(step);
            let t = 2.0 * std::f64::consts::PI * (q.x - (l + r) / 2.0) / w;
            // Above the armpit the crown narrows to a cap over the top of the arm, its top on the
            // arm's line beside the shoulder point it is sewn to.
            let cap = if q.y < under { 0.15 + 0.85 * (q.y - top) / (under - top).max(1e-9) } else { 1.0 };
            let radius = w / (2.0 * std::f64::consts::PI) * cap;
            [x_at + side * radius * t.cos(), toward * radius * t.sin(), z]
        }));
    }
    let placed: Vec<Placed> = copies
        .iter()
        .zip(&place)
        .map(|(c, f)| -> Result<Placed> {
            let f = f.as_ref().ok_or_else(|| bad(C, "a piece could not be placed on the body"))?;
            Ok((&c.panel, f.as_ref()))
        })
        .collect::<Result<_>>()?;
    let mut cloth = Cloth::new(&placed, Stiffness::default());
    // Each seam: the particles along its two runs paired by how far along the run they are, each
    // particle of the side with fewer sewn once (pairing the particles nearest points along the
    // runs sewed one particle to two a step apart where two edges slant differently, and the seam
    // could not close).
    let mut set_in = Vec::new();
    for sm in &seams {
        let (pa, mut pb) = (sm.pa.clone(), sm.pb.clone());
        if pa.len() < 2 || pb.len() < 2 {
            continue;
        }
        if !sm.ordered && (pa[0].y - pb[0].y).abs() > (pa[0].y - pb[pb.len() - 1].y).abs() {
            pb.reverse();
        }
        let (on_a, on_b) = (along(&copies[sm.a].panel, &pa, step), along(&copies[sm.b].panel, &pb, step));
        let (few, many, swap) = if on_a.len() <= on_b.len() { (&on_a, &on_b, false) } else { (&on_b, &on_a, true) };
        // In order along the run, each partner taken once: the next one not taken that is
        // nearest along it.
        let mut next = 0;
        for &(t, i) in few {
            while next + 1 < many.len() && (many[next + 1].0 - t).abs() <= (many[next].0 - t).abs() {
                next += 1;
            }
            let Some(&(_, j)) = many.get(next) else { break };
            next += 1;
            let (x, y) = if swap { (j, i) } else { (i, j) };
            let (u, v) = (x + cloth.starts[sm.a], y + cloth.starts[sm.b]);
            // A dart's point is one particle, sewn to nothing.
            if u != v {
                if (role(sm.a) == Role::Arm) != (role(sm.b) == Role::Arm) {
                    set_in.push(cloth.seams.len());
                }
                cloth.sew(u, v);
            }
        }
    }
    // Forty rounds of corrections a substep at the default precision, more the finer it is (the
    // cloth gets softer the more particles there are), never fewer: fewer let a ring of cloth
    // stretch round the hips and slide off (at ten, a skirt that fits fell to the floor; at 26,
    // the 25 mm cloth of a pair of trousers, as heavy as two skirts, slid down the legs).
    // Skirts and trousers, which hang from the waist, grip the body; a bodice and sleeves hang
    // from the shoulders, and gripping pinched them in the armpit (the tightest place 261% → 794%).
    for (c, copy) in copies.iter().enumerate() {
        if matches!(role(c), Role::Upper | Role::Arm) {
            let start = cloth.starts[c] as usize;
            cloth.gripping[start..start + copy.panel.points.len()].fill(false);
        }
    }
    let iterations = (40.0 * (20.0 / step).powi(2)).round().max(40.0) as usize;
    let darts = seams.iter().filter(|s| s.ordered).count();
    let sleeves: Vec<u32> = (0..copies.len())
        .filter(|c| role(*c) == Role::Arm)
        .flat_map(|c| {
            let start = cloth.starts[c];
            (0..copies[c].panel.points.len() as u32).map(move |i| start + i)
        })
        .collect();
    let grains = copies.iter().map(|c| (parts[c.piece].fabric.clone(), c.grain)).collect();
    Ok(Setup { cloth, avatar, steps, iterations, waist, pieces: parts.len(), darts, sleeves, set_in, grains })
}

fn make(s: &mut Session, p: &Value) -> Result<Value> {
    let mut setup = setup(s, p)?;
    setup.run(&mut |_| true);
    finish(s, p, setup)
}

/// The draped cloth's measures, files and drawing (see `drape.make`).
pub fn finish(s: &mut Session, p: &Value, setup: Setup) -> Result<Value> {
    const C: &str = "drape.make";
    let Setup { cloth, avatar, waist, pieces, darts, grains, .. } = setup;
    let (top, hem) = cloth.positions.iter().fold((f64::NEG_INFINITY, f64::INFINITY), |(t, h), q| (t.max(q[2]), h.min(q[2])));
    let r1 = |v: f64| (v * 10.0).round() / 10.0;
    let mut out = json!({
        "pieces": pieces,
        "darts": darts,
        "particles": cloth.positions.len(),
        "top": r1(top),
        "hem": r1(hem),
        "waist": r1(waist),
        "worstStretch": r1(cloth.worst_stretch() * 100.0),
        "widestSeam": r1(cloth.widest_seam()),
    });
    let tight = cloth.tightness();
    out["tightest"] = json!(r1(tight.iter().copied().fold(0.0, f64::max) * 100.0));
    let (fit, message) = pf_core::view3d::fit_message(cloth.worst_stretch(), cloth.widest_seam(), tight.iter().copied().fold(0.0, f64::max));
    out["fit"] = json!(format!("{fit:?}").to_lowercase());
    out["fitMessage"] = json!(message);
    // The draped cloth as a file: OBJ (the cloth), or glTF (the avatar and the cloth in its
    // colours), by `format`, else the path's extension.
    let path = str_param(p, "path");
    let format = match (str_param(p, "format"), path) {
        (Some(f), _) => Some(f.to_ascii_lowercase()),
        (None, Some(path)) => Some(if super::fileio::extension(path).eq_ignore_ascii_case("glb") { "glb".into() } else { "obj".into() }),
        (None, None) => None,
    };
    if let Some(format) = format {
        let bytes = match format.as_str() {
            "obj" => {
                let mesh = pf_core::avatar::Mesh { vertices: cloth.positions.clone(), triangles: cloth.triangles.clone() };
                pf_core::avatar::to_obj(&mesh, "drape").into_bytes()
            }
            "glb" => {
                let printed = match str_param(p, "colour") {
                    None | Some("fabric") => prints(&s.doc()?.doc, &cloth, &grains, FABRIC_PIXEL_MM),
                    Some("fit") => None,
                    Some(_) => return Err(bad(C, "colour: \"fabric\" or \"fit\"")),
                };
                glb(&avatar, &cloth, &tight, printed.as_ref())?
            }
            _ => return Err(bad(C, "format: \"obj\" or \"glb\"")),
        };
        match path {
            Some(path) => {
                super::fileio::write_file(path, &bytes)?;
                out["path"] = json!(path);
            }
            None if format == "obj" => out["data"] = json!(String::from_utf8_lossy(&bytes)),
            None => out["dataBase64"] = json!(vectorcraft_format::base64_encode(&bytes)),
        }
    }
    if p.get("draw").and_then(Value::as_bool) != Some(false) {
        let (layer, front, side, fabric) = draw_views(s, p, &avatar, &cloth, &tight, &grains)?;
        out["layer"] = json!(layer.0);
        out["front"] = json!(front.0);
        out["side"] = json!(side.0);
        if let Some(fabric) = fabric {
            out["fabric"] = json!(fabric.0);
        }
    }
    Ok(out)
}

/// The skin's colour, in the drape view and the glTF file.
const SKIN: [u8; 3] = [214, 192, 170];

/// Each particle's colour: the tightness of the triangles round it, averaged.
fn particle_colours(cloth: &Cloth, tight: &[f64]) -> Vec<[u8; 3]> {
    let mut sum = vec![(0.0, 0usize); cloth.positions.len()];
    for (t, tri) in tight.iter().zip(&cloth.triangles) {
        for i in tri {
            if let Some(s) = sum.get_mut(*i as usize) {
                s.0 += t;
                s.1 += 1;
            }
        }
    }
    sum.iter().map(|(t, n)| pf_core::view3d::tightness_colour(if *n > 0 { t / *n as f64 } else { 0.0 })).collect()
}

/// The avatar and the draped cloth as a binary glTF file, the cloth coloured by how tight it is
/// (as the drape view colours it), or with `prints` in its fabric photos (PF-10.18): a part for
/// each photo ("Drape: <fabric>"), its places on the photo in photos across and down (repeating),
/// and one for the rest in the fabric view's grey.
fn glb(avatar: &pf_core::avatar::Avatar, cloth: &Cloth, tight: &[f64], prints: Option<&Prints>) -> Result<Vec<u8>> {
    use pf_core::gltf::{Part, Texture, linear, linear_rgb, to_glb};
    let gltf = |e: pf_core::gltf::GltfError| bad("drape.make", e.to_string());
    let skin = [linear(SKIN[0]), linear(SKIN[1]), linear(SKIN[2]), 1.0];
    let avatar_part = Part {
        name: "Avatar",
        vertices: &avatar.mesh.vertices,
        triangles: &avatar.mesh.triangles,
        colours: None,
        base: skin,
        double_sided: false,
        texture: None,
    };
    let Some(pr) = prints else {
        let colours: Vec<[u8; 3]> = particle_colours(cloth, tight).into_iter().map(linear_rgb).collect();
        let drape = Part {
            name: "Drape",
            vertices: &cloth.positions,
            triangles: &cloth.triangles,
            colours: Some(&colours),
            base: [1.0; 4],
            double_sided: true,
            texture: None,
        };
        return to_glb(&[avatar_part, drape]).map_err(gltf);
    };
    let of = |want: Option<u32>| -> Vec<[u32; 3]> { cloth.triangles.iter().zip(&pr.fabric).filter(|(_, f)| **f == want).map(|(t, _)| *t).collect() };
    let mut pngs = Vec::new();
    let mut uvs = Vec::new();
    let mut tris = Vec::new();
    let mut names = Vec::new();
    for (i, sw) in pr.swatches.iter().enumerate() {
        let t = of(Some(i as u32));
        if t.is_empty() {
            continue;
        }
        let img = image::RgbImage::from_raw(sw.width, sw.height, sw.rgb.clone())
            .ok_or_else(|| EngineError::Other("a fabric photo is the wrong size".into()))?;
        // Each side the nearest power of two (up to 2048): a repeating texture of any other size
        // can't repeat in WebGL 1 viewers (its places are in photos, so they stay as they are).
        let pow2 = |n: u32| {
            let up = n.next_power_of_two();
            (if up - n > n - up / 2 && up > 1 { up / 2 } else { up }).clamp(1, 2048)
        };
        let img = image::imageops::resize(&img, pow2(sw.width), pow2(sw.height), image::imageops::FilterType::Triangle);
        let mut png = Vec::new();
        img.write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png).map_err(|e| EngineError::Other(format!("a fabric photo: {e}")))?;
        let (pw, ph) = (sw.width as f64 * sw.mm, sw.height as f64 * sw.mm);
        uvs.push(pr.at.iter().map(|a| [a[0] / pw, a[1] / ph]).collect::<Vec<_>>());
        pngs.push(png);
        tris.push(t);
        let name = pr.names.get(i).map(|n| n.trim()).filter(|n| !n.is_empty()).unwrap_or("Main");
        names.push(format!("Drape: {name}"));
    }
    let rest = of(None);
    let grey = [linear(UNPRINTED[0]), linear(UNPRINTED[1]), linear(UNPRINTED[2]), 1.0];
    let mut parts = vec![avatar_part];
    for k in 0..tris.len() {
        parts.push(Part {
            name: &names[k],
            vertices: &cloth.positions,
            triangles: &tris[k],
            colours: None,
            base: [1.0; 4],
            double_sided: true,
            texture: Some(Texture { uv: &uvs[k], png: &pngs[k] }),
        });
    }
    if !rest.is_empty() {
        parts.push(Part {
            name: "Drape",
            vertices: &cloth.positions,
            triangles: &rest,
            colours: None,
            base: grey,
            double_sided: true,
            texture: None,
        });
    }
    to_glb(&parts).map_err(gltf)
}

/// The layer the drape view is drawn on.
const LAYER: &str = "Drape";
/// The drape view's pixel, mm: fine enough to see folds at true scale on the page.
const PIXEL_MM: f64 = 2.0;
/// The view in the fabric's pixel, mm: fine enough to see a print.
const FABRIC_PIXEL_MM: f64 = 1.0;

/// A picture as PNG.
fn png(pic: &pf_core::view3d::Picture) -> Result<Vec<u8>> {
    let img = image::RgbaImage::from_raw(pic.width, pic.height, pic.rgba.clone())
        .ok_or_else(|| EngineError::Other("the drape view's picture is the wrong size".into()))?;
    let mut buf = Vec::new();
    img.write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Png).map_err(|e| EngineError::Other(format!("the drape view: {e}")))?;
    Ok(buf)
}

/// The cloth in the fabric view where a piece has no photo to be seen in.
const UNPRINTED: [u8; 3] = [200, 200, 205];

/// The draped cloth printed with the document's fabric photos (PF-10.16): each particle's place on
/// its fabric (mm across and down the grain from the pattern's origin), each triangle's photo
/// (into `swatches`: the photo named for its piece's fabric, else the first), and the photos that
/// could be read, about `mm` a pixel, with their fabrics' names.
struct Prints {
    at: Vec<[f64; 2]>,
    fabric: Vec<Option<u32>>,
    swatches: Vec<pf_core::view3d::Swatch>,
    names: Vec<String>,
}

/// The cloth's [`Prints`]; `None` without fabric photos.
fn prints(doc: &Document, cloth: &Cloth, grains: &[(String, Pt)], mm: f64) -> Option<Prints> {
    let photos = super::fabric::photos(doc);
    if photos.is_empty() {
        return None;
    }
    let read: Vec<Option<pf_core::view3d::Swatch>> = photos.iter().map(|(_, f, key)| super::fabric::swatch(doc, f, key, mm)).collect();
    let readable: Vec<usize> = (0..read.len()).filter(|i| read[*i].is_some()).collect();
    let swatches: Vec<pf_core::view3d::Swatch> = readable.iter().filter_map(|i| read[*i].clone()).collect();
    let names = readable.iter().map(|i| photos[*i].1.name.clone()).collect();
    let first = readable.first().map(|_| 0u32);
    let of_panel: Vec<Option<u32>> = grains
        .iter()
        .map(|(name, _)| {
            let named = photos.iter().position(|(_, f, _)| f.name.trim().eq_ignore_ascii_case(name.trim()));
            named.and_then(|i| readable.iter().position(|r| *r == i)).map(|i| i as u32).or(first)
        })
        .collect();
    let at = cloth
        .on_pattern()
        .iter()
        .map(|(q, panel)| {
            let g = grains.get(*panel as usize).map_or(Pt::new(0.0, 1.0), |g| g.1);
            [q.x * g.y - q.y * g.x, q.x * g.x + q.y * g.y]
        })
        .collect();
    let fabric = cloth
        .triangles
        .iter()
        .map(|t| cloth.on_pattern().get(t[0] as usize).and_then(|(_, panel)| of_panel.get(*panel as usize).copied().flatten()))
        .collect();
    Some(Prints { at, fabric, swatches, names })
}

/// The draped cloth seen from the front in the document's fabric photos (PF-10.16), at
/// [`FABRIC_PIXEL_MM`] (see [`Prints`]); `None` without photos.
fn fabric_view(doc: &Document, avatar: &pf_core::avatar::Avatar, cloth: &Cloth, grains: &[(String, Pt)]) -> Option<pf_core::view3d::Picture> {
    use pf_core::view3d::{Print, Shape, Side, draw};
    let Prints { at, fabric, swatches: list, .. } = prints(doc, cloth, grains, FABRIC_PIXEL_MM)?;
    let skin = vec![SKIN; avatar.mesh.triangles.len()];
    let plain = vec![UNPRINTED; cloth.triangles.len()];
    let shapes = [
        Shape { points: &avatar.mesh.vertices, triangles: &avatar.mesh.triangles, colours: &skin, smooth: true, print: None },
        Shape {
            points: &cloth.positions,
            triangles: &cloth.triangles,
            colours: &plain,
            smooth: true,
            print: Some(Print { at: &at, fabric: &fabric, swatches: &list }),
        },
    ];
    draw(&shapes, Side::Front, FABRIC_PIXEL_MM)
}

/// Draw the avatar and the draped cloth seen from the front and the left side, side by side at
/// true scale, on the layer "Drape" (replacing it), and with fabric photos the front again in
/// them: → (layer, front, side, in the fabric).
fn draw_views(
    s: &mut Session,
    p: &Value,
    avatar: &pf_core::avatar::Avatar,
    cloth: &Cloth,
    tight: &[f64],
    grains: &[(String, Pt)],
) -> Result<(NodeId, NodeId, NodeId, Option<NodeId>)> {
    use pf_core::view3d::{Shape, Side, draw, tightness_colour};
    const C: &str = "drape.make";
    let skin = vec![SKIN; avatar.mesh.triangles.len()];
    let colours: Vec<[u8; 3]> = tight.iter().map(|t| tightness_colour(*t)).collect();
    let shapes = [
        Shape { points: &avatar.mesh.vertices, triangles: &avatar.mesh.triangles, colours: &skin, smooth: true, print: None },
        Shape { points: &cloth.positions, triangles: &cloth.triangles, colours: &colours, smooth: true, print: None },
    ];
    let (front, side) = (draw(&shapes, Side::Front, PIXEL_MM), draw(&shapes, Side::Left, PIXEL_MM));
    let (Some(front), Some(side)) = (front, side) else { return Err(bad(C, "nothing to draw")) };
    let doc = &s.doc()?.doc;
    let printed = fabric_view(doc, avatar, cloth, grains);
    let origin = match p.get("origin") {
        None | Some(Value::Null) => {
            let b = doc.art_bounds();
            let at = doc.artboards.first().map_or(Point::ZERO, |ab| ab.rect.origin());
            Point::new(b.map_or(at.x, |b| b.x1) + pf_core::mm_to_pt(100.0), b.map_or(at.y, |b| b.y0))
        }
        Some(v) => {
            let [x, y] = finite_numbers::<2>(v).ok_or_else(|| bad(C, "origin: [x, y] in document points"))?;
            Point::new(x, y)
        }
    };
    let (fpng, spng) = (png(&front)?, png(&side)?);
    let printed = match printed {
        Some(pic) => Some((png(&pic)?, pic)),
        None => None,
    };
    // The side view 100 mm right of the front view; both from the same height (their tops are
    // the same point of the body), so they line up across.
    let side_x = pf_core::mm_to_pt(front.width as f64 * front.mm + 100.0);
    let dy = pf_core::mm_to_pt(front.top - side.top);
    // In the fabric: 100 mm right of the side view, from the same height.
    let fabric_x = side_x + pf_core::mm_to_pt(side.width as f64 * side.mm + 100.0);
    s.edit("Drape on Avatar", |d, sel| {
        d.layers.retain(|l| l.name.as_deref() != Some(LAYER));
        let image = |d: &mut Document, pic: &pf_core::view3d::Picture, bytes: Vec<u8>, at: Point, name: &str| {
            let key = super::menucmds::unique_key(d, "drape");
            d.images.insert(key.clone(), vectorcraft_doc::ImageBlob::new("image/png", bytes));
            let k = pf_core::mm_to_pt(pic.mm);
            let xf = vectorcraft_geom::Affine::new([k, 0.0, 0.0, k, at.x, at.y]);
            let mut n = vectorcraft_doc::Node::new(
                d.alloc_id(),
                NodeKind::Image(Box::new(vectorcraft_doc::ImageObject {
                    key,
                    width: pic.width,
                    height: pic.height,
                    xf,
                    link: None,
                    placement: Default::default(),
                })),
            );
            n.name = Some(name.into());
            n
        };
        let f = image(d, &front, fpng.clone(), origin, "Drape front");
        let sd = image(d, &side, spng.clone(), Point::new(origin.x + side_x, origin.y + dy), "Drape side");
        let fb = printed.as_ref().map(|(bytes, pic)| {
            image(d, pic, bytes.clone(), Point::new(origin.x + fabric_x, origin.y + pf_core::mm_to_pt(front.top - pic.top)), "Drape in fabric")
        });
        let ids = (f.id, sd.id, fb.as_ref().map(|n| n.id));
        let mut l = vectorcraft_doc::Node::layer(d.alloc_id(), LAYER, vectorcraft_doc::LayerColor::Preset(4));
        let id = l.id;
        if let Some(ch) = l.children_mut() {
            *ch = [Some(f), Some(sd), fb].into_iter().flatten().map(std::sync::Arc::new).collect();
        }
        d.layers.push(std::sync::Arc::new(l));
        sel.set([ids.0, ids.1].into_iter().chain(ids.2));
        Ok((id, ids.0, ids.1, ids.2))
    })
}
