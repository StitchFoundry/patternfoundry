//! The drape view (PF-10.4): draped cloth and the avatar drawn as a picture, seen from the front
//! or the side at a true scale, each triangle lit by a light over the viewer's shoulder and the
//! nearer triangle in front (a depth buffer), the cloth coloured by how tight it is. A smooth
//! shape (PF-10.15) is lit as a curved surface: each point's normal the average of its triangles',
//! blended across each triangle (Gouraud's shading, the light worked out at every pixel from the
//! blended normal), so a body of rings reads as round, not faceted.
//!
//! Tight is how much a triangle of cloth is larger than it was cut (its area over its area on
//! the pattern): at or under its cut size it is blue, half again of [`TIGHT`] green, and
//! [`TIGHT`] or more red, the colours run between (the way fit software shows strain on cloth,
//! as a colour map from loose to tight).

/// Stretched this much (a share of its area) a triangle of cloth is shown red.
pub const TIGHT: f64 = 0.10;
/// Most pixels a view takes: a bound on memory.
pub const MAX_PIXELS: u64 = 16_000_000;

/// Which way the body is seen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    /// From in front of it (looking along -y): the body's left on the picture's right.
    Front,
    /// From its left side (looking along -x): its front on the picture's right.
    Left,
}

/// One thing to draw: its points (mm), its triangles and each triangle's colour (RGB); `smooth`
/// lights it as a curved surface rather than flat triangles.
pub struct Shape<'a> {
    pub points: &'a [[f64; 3]],
    pub triangles: &'a [[u32; 3]],
    pub colours: &'a [[u8; 3]],
    pub smooth: bool,
    /// Where it is printed with a fabric (PF-10.16); else in its colours.
    pub print: Option<Print<'a>>,
}

/// A fabric's picture (PF-10.16): `width` × `height` pixels of RGB, each `mm` across, repeating
/// both ways as the fabric does.
#[derive(Clone, Debug, PartialEq)]
pub struct Swatch {
    pub width: u32,
    pub height: u32,
    pub rgb: Vec<u8>,
    pub mm: f64,
}

impl Swatch {
    /// Its colour at `u` across and `v` down the fabric (mm).
    fn at(&self, u: f64, v: f64) -> Option<[u8; 3]> {
        if self.width == 0 || self.height == 0 || self.mm.is_nan() || self.mm <= 0.0 {
            return None;
        }
        let x = (u / self.mm).floor().rem_euclid(self.width as f64) as usize;
        let y = (v / self.mm).floor().rem_euclid(self.height as f64) as usize;
        let i = (y.min(self.height as usize - 1) * self.width as usize + x.min(self.width as usize - 1)) * 3;
        Some([*self.rgb.get(i)?, *self.rgb.get(i + 1)?, *self.rgb.get(i + 2)?])
    }
}

/// A shape printed with fabrics: each point's place on its fabric (mm across and down, the
/// lengthwise grain down), and each triangle's fabric (into `swatches`; none: its own colour).
#[derive(Clone, Copy, Debug)]
pub struct Print<'a> {
    pub at: &'a [[f64; 2]],
    pub fabric: &'a [Option<u32>],
    pub swatches: &'a [Swatch],
}

/// Each point's normal in the view (across, up, toward the viewer): the sum of its triangles'
/// normals, each as long as its triangle is large (so the larger count for more).
fn normals(points: &[(f64, f64, f64)], triangles: &[[u32; 3]]) -> Vec<(f64, f64, f64)> {
    let mut out = vec![(0.0, 0.0, 0.0); points.len()];
    for tri in triangles {
        let [a, b, c] = tri.map(|i| i as usize);
        let (Some(pa), Some(pb), Some(pc)) = (points.get(a), points.get(b), points.get(c)) else { continue };
        let (e1, e2) = ((pb.0 - pa.0, pb.1 - pa.1, pb.2 - pa.2), (pc.0 - pa.0, pc.1 - pa.1, pc.2 - pa.2));
        let n = (e1.1 * e2.2 - e1.2 * e2.1, e1.2 * e2.0 - e1.0 * e2.2, e1.0 * e2.1 - e1.1 * e2.0);
        for k in [a, b, c] {
            out[k] = (out[k].0 + n.0, out[k].1 + n.1, out[k].2 + n.2);
        }
    }
    out
}

/// A picture: `width` × `height` pixels of RGBA (straight alpha, transparent where nothing is),
/// each pixel `mm` across, its top-left corner at `left` across and `top` up (mm, in the view's
/// own across and up).
#[derive(Clone, Debug, PartialEq)]
pub struct Picture {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
    pub mm: f64,
    pub left: f64,
    pub top: f64,
}

/// Where point `p` is seen: (across, up, depth toward the viewer).
fn seen(p: [f64; 3], side: Side) -> (f64, f64, f64) {
    match side {
        Side::Front => (p[0], p[2], p[1]),
        Side::Left => (p[1], p[2], p[0]),
    }
}

/// The colour of tightness `t` (a triangle's area over its cut area, less 1).
pub fn tightness_colour(t: f64) -> [u8; 3] {
    let k = (t / TIGHT).clamp(0.0, 1.0);
    // Blue to green to red.
    let (r, g, b) = if k < 0.5 {
        let u = k * 2.0;
        (0.0, u, 1.0 - u)
    } else {
        let u = (k - 0.5) * 2.0;
        (u, 1.0 - u, 0.0)
    };
    [(r * 220.0 + 20.0) as u8, (g * 200.0 + 30.0) as u8, (b * 220.0 + 20.0) as u8]
}

/// The worst stretch (a share, 0.3 is 30%) over which a garment is called too tight.
pub const TOO_TIGHT_STRETCH: f64 = 0.30;
/// The widest gap (mm) a seam may be left open by before it is called not meeting.
pub const OPEN_SEAM_MM: f64 = 20.0;

/// How a draped garment fits, from the drape's measures: its [`fit_message`] level.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fit {
    /// Room to spare: nothing is cut tighter than the body.
    Loose,
    /// Close, some cloth `TIGHT` or more larger than cut, but nothing is strained.
    Snug,
    /// Stretched past [`TOO_TIGHT_STRETCH`].
    TooTight,
    /// A seam stays open past [`OPEN_SEAM_MM`].
    OpenSeam,
}

/// The fit and one line in plain words for the drape's `worst_stretch` (a share), `widest_seam`
/// (mm) and `tightest` (a share: the most a triangle of cloth is larger than cut). A seam that
/// does not meet is named first: it usually means a wrong measurement.
pub fn fit_message(worst_stretch: f64, widest_seam: f64, tightest: f64) -> (Fit, String) {
    if widest_seam > OPEN_SEAM_MM {
        let m = format!("Seams do not meet (up to {} mm apart): check your measurements", widest_seam.round());
        return (Fit::OpenSeam, m);
    }
    if worst_stretch > TOO_TIGHT_STRETCH {
        let m = format!("Too tight: the cloth is pulled {}% past its cut size: check your measurements", (worst_stretch * 100.0).round());
        return (Fit::TooTight, m);
    }
    if tightest >= TIGHT {
        return (Fit::Snug, "Snug: close to the body, with little room to spare".into());
    }
    (Fit::Loose, "Fits with room to spare".into())
}

/// Draw `shapes` seen from `side`, `mm` a pixel (at least a tenth of a millimetre).
pub fn draw(shapes: &[Shape], side: Side, mm: f64) -> Option<Picture> {
    if !(mm.is_finite() && mm >= 0.1) {
        return None;
    }
    let all = shapes.iter().flat_map(|s| s.points.iter().map(|p| seen(*p, side)));
    let (l, r, b, t) = all.fold((f64::INFINITY, f64::NEG_INFINITY, f64::INFINITY, f64::NEG_INFINITY), |(l, r, b, t), q| {
        (l.min(q.0), r.max(q.0), b.min(q.1), t.max(q.1))
    });
    if !(l.is_finite() && r > l && t > b) {
        return None;
    }
    let (w, h) = (((r - l) / mm).ceil() as u64 + 1, ((t - b) / mm).ceil() as u64 + 1);
    if w * h > MAX_PIXELS {
        return None;
    }
    let (w, h) = (w as usize, h as usize);
    let mut rgba = vec![0u8; w * h * 4];
    let mut depth = vec![f64::NEG_INFINITY; w * h];
    // The light: from over the viewer's left shoulder, a little above.
    let light = {
        let v: [f64; 3] = [-0.35, 0.45, 0.82];
        let n = v[0] * v[0] + v[1] * v[1] + v[2] * v[2];
        [v[0] / n.sqrt(), v[1] / n.sqrt(), v[2] / n.sqrt()]
    };
    let lit_by = |n: (f64, f64, f64)| {
        let len = (n.0 * n.0 + n.1 * n.1 + n.2 * n.2).sqrt();
        if len < 1e-12 { 1.0 } else { 0.35 + 0.65 * ((n.0 * light[0] + n.1 * light[1] + n.2 * light[2]) / len).abs() }
    };
    for s in shapes {
        let vertex_normals = s.smooth.then(|| normals(&s.points.iter().map(|p| seen(*p, side)).collect::<Vec<_>>(), s.triangles));
        for (k, tri) in s.triangles.iter().enumerate() {
            let Some(c) = s.colours.get(k) else { continue };
            let v: Vec<(f64, f64, f64)> = tri.iter().filter_map(|i| s.points.get(*i as usize)).map(|p| seen(*p, side)).collect();
            let [a, bb, cc] = v[..] else { continue };
            // In the view's own across, up and toward-the-viewer: its normal, either way out (the
            // cloth has two sides).
            let (e1, e2) = ((bb.0 - a.0, bb.1 - a.1, bb.2 - a.2), (cc.0 - a.0, cc.1 - a.1, cc.2 - a.2));
            let n = (e1.1 * e2.2 - e1.2 * e2.1, e1.2 * e2.0 - e1.0 * e2.2, e1.0 * e2.1 - e1.1 * e2.0);
            let len = (n.0 * n.0 + n.1 * n.1 + n.2 * n.2).sqrt();
            if len < 1e-12 {
                continue;
            }
            let flat = lit_by(n);
            // Its fabric, and its corners' places on it.
            let printed = s.print.and_then(|pr| {
                let sw = pr.swatches.get(pr.fabric.get(k).copied().flatten()? as usize)?;
                let [u0, u1, u2] = tri.map(|i| pr.at.get(i as usize).copied());
                Some((sw, u0?, u1?, u2?))
            });
            let corner = |i: u32| vertex_normals.as_ref().and_then(|v| v.get(i as usize).copied());
            let smooth = match (corner(tri[0]), corner(tri[1]), corner(tri[2])) {
                (Some(n0), Some(n1), Some(n2)) => Some((n0, n1, n2)),
                _ => None,
            };
            let px = |q: (f64, f64, f64)| ((q.0 - l) / mm, (t - q.1) / mm, q.2);
            let (p0, p1, p2) = (px(a), px(bb), px(cc));
            let area = (p1.0 - p0.0) * (p2.1 - p0.1) - (p2.0 - p0.0) * (p1.1 - p0.1);
            if area.abs() < 1e-12 {
                continue;
            }
            let (x0, x1) = (p0.0.min(p1.0).min(p2.0).floor().max(0.0) as usize, (p0.0.max(p1.0).max(p2.0).ceil() as usize).min(w - 1));
            let (y0, y1) = (p0.1.min(p1.1).min(p2.1).floor().max(0.0) as usize, (p0.1.max(p1.1).max(p2.1).ceil() as usize).min(h - 1));
            for y in y0..=y1 {
                for x in x0..=x1 {
                    let (fx, fy) = (x as f64 + 0.5, y as f64 + 0.5);
                    let w0 = ((p1.0 - fx) * (p2.1 - fy) - (p2.0 - fx) * (p1.1 - fy)) / area;
                    let w1 = ((p2.0 - fx) * (p0.1 - fy) - (p0.0 - fx) * (p2.1 - fy)) / area;
                    let w2 = 1.0 - w0 - w1;
                    if w0 < 0.0 || w1 < 0.0 || w2 < 0.0 {
                        continue;
                    }
                    let z = w0 * p0.2 + w1 * p1.2 + w2 * p2.2;
                    let at = y * w + x;
                    if z <= depth[at] {
                        continue;
                    }
                    depth[at] = z;
                    let shade = match smooth {
                        Some((n0, n1, n2)) => {
                            lit_by((w0 * n0.0 + w1 * n1.0 + w2 * n2.0, w0 * n0.1 + w1 * n1.1 + w2 * n2.1, w0 * n0.2 + w1 * n1.2 + w2 * n2.2))
                        }
                        None => flat,
                    };
                    let colour = printed
                        .and_then(|(sw, u0, u1, u2)| sw.at(w0 * u0[0] + w1 * u1[0] + w2 * u2[0], w0 * u0[1] + w1 * u1[1] + w2 * u2[1]))
                        .unwrap_or(*c);
                    for (j, ch) in colour.iter().enumerate() {
                        rgba[at * 4 + j] = (*ch as f64 * shade).round().clamp(0.0, 255.0) as u8;
                    }
                    rgba[at * 4 + 3] = 255;
                }
            }
        }
    }
    Some(Picture { width: w as u32, height: h as u32, rgba, mm, left: l, top: t })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A unit square of two triangles at depth `d`, `size` mm, in the x-z plane (facing front).
    fn square(size: f64, d: f64) -> (Vec<[f64; 3]>, Vec<[u32; 3]>) {
        (vec![[0.0, d, 0.0], [size, d, 0.0], [size, d, size], [0.0, d, size]], vec![[0, 1, 2], [0, 2, 3]])
    }

    #[test]
    fn a_square_is_drawn_at_its_true_size_and_the_nearer_in_front() {
        let (p, t) = square(100.0, 0.0);
        let (q, u) = square(100.0, 10.0);
        let red = [[200, 0, 0]; 2];
        let blue = [[0, 0, 200]; 2];
        let pic = draw(
            &[
                Shape { points: &p, triangles: &t, colours: &red, smooth: false, print: None },
                Shape { points: &q, triangles: &u, colours: &blue, smooth: false, print: None },
            ],
            Side::Front,
            1.0,
        )
        .unwrap();
        // 100 mm at 1 mm a pixel: 101 pixels across and up.
        assert_eq!((pic.width, pic.height), (101, 101));
        // The middle pixel is the nearer square's (blue, at depth 10 toward the viewer), lit.
        let mid = ((50 * pic.width + 50) * 4) as usize;
        let px = &pic.rgba[mid..mid + 4];
        assert!(px[2] > 60 && px[0] == 0 && px[3] == 255, "{px:?}");
        // Seen from the side, a square in the x-z plane is a line: nothing much drawn.
        let side = draw(&[Shape { points: &p, triangles: &t, colours: &red, smooth: false, print: None }], Side::Left, 1.0);
        assert!(side.is_none_or(|s| s.rgba.chunks(4).filter(|c| c[3] == 255).count() < 200));
        assert!(draw(&[Shape { points: &p, triangles: &t, colours: &red, smooth: false, print: None }], Side::Front, 0.0).is_none());
    }

    /// Smooth (PF-10.15): a bent strip of two faces, each flat, shades evenly across where they meet
    /// when smooth (the shade changing gradually), in a step when not.
    #[test]
    fn a_smooth_shape_shades_gradually_across_its_edges() {
        // Two squares meeting at x = 50 at an angle (a ridge toward the viewer).
        let p = vec![[0.0, 0.0, 0.0], [50.0, 30.0, 0.0], [100.0, 0.0, 0.0], [0.0, 0.0, 100.0], [50.0, 30.0, 100.0], [100.0, 0.0, 100.0]];
        let t = vec![[0, 1, 4], [0, 4, 3], [1, 2, 5], [1, 5, 4]];
        let grey = [[200, 200, 200]; 4];
        let row = |smooth| {
            let pic = draw(&[Shape { points: &p, triangles: &t, colours: &grey, smooth, print: None }], Side::Front, 1.0).unwrap();
            // The drawn pixels only (the background is 0).
            (0..pic.width).map(|x| pic.rgba[((50 * pic.width + x) * 4) as usize] as i32).filter(|v| *v > 0).collect::<Vec<_>>()
        };
        let (flat, round) = (row(false), row(true));
        // Flat: one shade each side of the ridge, a step at it. Smooth: no step that large.
        let step = |r: &[i32]| r.windows(2).map(|w| (w[1] - w[0]).abs()).max().unwrap_or(0);
        assert!(step(&flat) > 10, "{flat:?}");
        assert!(step(&round) < step(&flat) / 2, "{round:?}");
    }

    #[test]
    fn tightness_runs_from_blue_to_red() {
        let (loose, mid, tight) = (tightness_colour(-0.05), tightness_colour(TIGHT / 2.0), tightness_colour(TIGHT * 2.0));
        assert!(loose[2] > loose[0] && loose[2] > loose[1]);
        assert!(mid[1] > mid[0] && mid[1] > mid[2]);
        assert!(tight[0] > tight[1] && tight[0] > tight[2]);
    }

    /// The fit line (issue #157): a seam gap over 20 mm or a stretch over 30% is named, a seam
    /// first; at the limits it is not; a skirt with some cloth 10% larger is snug, else loose.
    #[test]
    fn the_fit_message_names_a_problem_past_its_thresholds() {
        assert_eq!(fit_message(0.05, 3.0, 0.02), (Fit::Loose, "Fits with room to spare".into()));
        assert_eq!(fit_message(0.05, 3.0, 0.10).0, Fit::Snug);
        assert_eq!(fit_message(0.30, 20.0, 0.50).0, Fit::Snug);
        let (fit, text) = fit_message(2.18, 10.0, 3.0);
        assert_eq!(fit, Fit::TooTight);
        assert!(text.contains("218%"), "{text}");
        let (fit, text) = fit_message(0.1, 56.4, 0.0);
        assert_eq!(fit, Fit::OpenSeam);
        assert!(text.contains("56 mm") && text.contains("measurements"), "{text}");
        // Both: the seam is named first.
        assert_eq!(fit_message(2.18, 56.0, 3.0).0, Fit::OpenSeam);
    }

    /// A printed shape (PF-10.16) shows its fabric at true size, repeating: a swatch of two 10 mm
    /// stripes (red, then white) across a 100 mm square shows five of each, each 10 mm wide.
    #[test]
    fn a_printed_shape_shows_its_fabric_at_true_size() {
        let (p, t) = square(100.0, 0.0);
        let grey = [[128, 128, 128]; 2];
        // Its places on the fabric: across as the square, down from its top.
        let at: Vec<[f64; 2]> = p.iter().map(|q| [q[0], 100.0 - q[2]]).collect();
        let swatch = Swatch { width: 2, height: 1, rgb: vec![200, 0, 0, 255, 255, 255], mm: 10.0 };
        let fabric = [Some(0), Some(0)];
        let print = Print { at: &at, fabric: &fabric, swatches: std::slice::from_ref(&swatch) };
        let pic = draw(&[Shape { points: &p, triangles: &t, colours: &grey, smooth: false, print: Some(print) }], Side::Front, 1.0).unwrap();
        let red = |x: u32| {
            let i = ((50 * pic.width + x) * 4) as usize;
            pic.rgba[i] > 100 && pic.rgba[i + 1] < 50
        };
        let stripes: Vec<bool> = (0..100).map(red).collect();
        // Lit, but a red pixel is red and a white one isn't; ten stripes, changing every 10 mm.
        let changes: Vec<usize> = (1..100).filter(|x| stripes[*x] != stripes[x - 1]).collect();
        assert_eq!(changes, vec![10, 20, 30, 40, 50, 60, 70, 80, 90], "{stripes:?}");
        assert!(stripes[5] && !stripes[15]);
        // Unprinted triangles keep their colours.
        let fabric = [None, None];
        let print = Print { at: &at, fabric: &fabric, swatches: std::slice::from_ref(&swatch) };
        let plain = draw(&[Shape { points: &p, triangles: &t, colours: &grey, smooth: false, print: Some(print) }], Side::Front, 1.0).unwrap();
        let i = ((50 * plain.width + 5) * 4) as usize;
        assert_eq!(plain.rgba[i], plain.rgba[i + 1]);
    }
}
