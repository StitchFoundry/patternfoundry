//! # PatternFoundry pattern core
//!
//! What makes a path a sewing-pattern piece, and the geometry drawn from it. Nothing here knows the
//! vector editor: the host flattens a piece's seam line into an [`Outline`] and draws the marks.
//!
//! * [`piece`] – the data a piece carries ([`Piece`]): name, cut count, fabric, seam allowance per
//!   edge, corner styles, notches, grainline; and [`PatternTag`], how the host marks the parts of a
//!   piece it draws.
//! * [`allowance`] – the cutting line: the seam line pushed out by each edge's allowance, with
//!   mitred or squared corners.
//! * [`marks`] – notches, the grainline arrow and the label text.
//! * [`measure`] – measurement sets (in mm, with formulas over them) and size charts; [`expr`]
//!   the formulas' small language; [`draft`] – path points placed by formulas, so a draft
//!   follows the measurements.
//! * [`block`] – blocks drafted from the measurements (the close-fitting bodice, the straight
//!   skirt, the one-piece sleeve, the trousers).
//! * [`design`] – design tools on an outline: pivot a dart elsewhere, split a piece in two.
//! * [`grade`] – grade rules: how a piece's points move from size to size.
//! * [`marker`] – marker making: every piece to cut laid on the fabric's width, closely.
//! * [`placement`] – embroidery and screen-print designs placed on a piece, at their true size.
//! * [`tiling`] – what a printed page of a tiled pattern carries: its name, overlap lines and the
//!   10 cm and 1 in test squares.
//!
//! **Units.** Allowances and mark sizes are stored in millimetres, the unit sewists measure in
//! (true scale is exact: [`MM_PER_PT`] is the definition, not a rounding). Positions (the
//! grainline's ends) are in the host document's coordinates, so they move with the piece. The
//! geometry functions take every length in the outline's own unit; the host converts.

pub mod allowance;
pub mod avatar;
pub mod block;
pub mod cloth;
pub mod design;
pub mod draft;
pub mod expr;
pub mod gltf;
pub mod grade;
pub mod marker;
pub mod marks;
pub mod measure;
pub mod piece;
pub mod placement;
pub mod swatch;
pub mod tiling;
pub mod view3d;

pub use allowance::{Outline, cutting_line};
pub use piece::{Corner, Drill, FabricPhoto, Grainline, Mark, Notch, NotchKind, PatternTag, Piece, Sewn};
pub use placement::{Placement, PlacementKind};

/// Millimetres in one PostScript point (1 pt = 1/72 in, 1 in = 25.4 mm, exactly).
pub const MM_PER_PT: f64 = 25.4 / 72.0;

/// Millimetres to points.
pub fn mm_to_pt(mm: f64) -> f64 {
    mm / MM_PER_PT
}

/// Points to millimetres.
pub fn pt_to_mm(pt: f64) -> f64 {
    pt * MM_PER_PT
}

/// A point or a vector in the plane (y down, as the host document).
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Pt {
    pub x: f64,
    pub y: f64,
}

impl Pt {
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
    pub fn dot(self, o: Pt) -> f64 {
        self.x * o.x + self.y * o.y
    }
    pub fn cross(self, o: Pt) -> f64 {
        self.x * o.y - self.y * o.x
    }
    pub fn len(self) -> f64 {
        self.x.hypot(self.y)
    }
    /// The unit vector, or `None` for a zero (or not finite) vector.
    pub fn unit(self) -> Option<Pt> {
        let l = self.len();
        (l.is_finite() && l > 1e-12).then(|| self * (1.0 / l))
    }
    pub fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite()
    }
}

impl std::ops::Add for Pt {
    type Output = Pt;
    fn add(self, o: Pt) -> Pt {
        Pt::new(self.x + o.x, self.y + o.y)
    }
}

impl std::ops::Sub for Pt {
    type Output = Pt;
    fn sub(self, o: Pt) -> Pt {
        Pt::new(self.x - o.x, self.y - o.y)
    }
}

impl std::ops::Mul<f64> for Pt {
    type Output = Pt;
    fn mul(self, k: f64) -> Pt {
        Pt::new(self.x * k, self.y * k)
    }
}

impl std::ops::Neg for Pt {
    type Output = Pt;
    fn neg(self) -> Pt {
        Pt::new(-self.x, -self.y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ten_centimetres_are_exactly_283_46_points() {
        assert!((mm_to_pt(100.0) - 7200.0 / 25.4).abs() < 1e-9);
        assert!((pt_to_mm(mm_to_pt(37.5)) - 37.5).abs() < 1e-12);
        assert_eq!(pt_to_mm(72.0), 25.4);
    }
}
