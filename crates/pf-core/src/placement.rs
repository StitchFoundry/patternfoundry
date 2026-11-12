//! Designs placed on a piece (PF-12): an embroidery design or a screen-print artwork laid on a
//! pattern piece to see its size and place before cutting.
//!
//! A placement keeps the design's own size in mm (read from its file, or given), a scale, a turn and
//! the place of its centre in the host document's coordinates, so it moves with the piece. Every
//! length the sewist sees is true: the size shown is the design's size times the scale, and the
//! outline drawn on the pattern measures exactly that when printed at 100%.

use serde::{Deserialize, Serialize};

use crate::{Pt, mm_to_pt};

/// Most placements on one piece.
pub const MAX_PLACEMENTS: usize = 100;
/// The widest or tallest a design can be, in mm (a bound for untrusted files).
pub const MAX_DESIGN_MM: f64 = 5000.0;
/// The smallest and largest scale of a design.
pub const SCALE_RANGE: std::ops::RangeInclusive<f64> = 0.01..=100.0;
/// How far a placement's crosshair reaches past the design's edge, in mm.
pub const CROSSHAIR_OVERSHOOT_MM: f64 = 5.0;
/// Millimetres in an inch.
pub const MM_PER_IN: f64 = 25.4;

/// What kind of design is placed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PlacementKind {
    /// A StitchFoundry embroidery design (PES, DST and the like).
    #[default]
    Embroidery,
    /// A ScreenFoundry artwork or separation (PNG, SVG).
    Screen,
}

impl PlacementKind {
    /// The verb for the cut list: what is done to the fabric.
    pub fn verb(self) -> &'static str {
        match self {
            PlacementKind::Embroidery => "Embroider",
            PlacementKind::Screen => "Screen print",
        }
    }
    /// The Foundry app that edits this kind of design.
    pub fn app(self) -> &'static str {
        match self {
            PlacementKind::Embroidery => "StitchFoundry",
            PlacementKind::Screen => "ScreenFoundry",
        }
    }
}

/// A design placed on a piece.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Placement {
    /// What it is called (the design's name).
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub kind: PlacementKind,
    /// The file it came from (a path or a file name), so it can be opened again to edit.
    #[serde(default)]
    pub source: String,
    /// The design's own width and height, in mm, before the scale.
    pub width: f64,
    pub height: f64,
    /// The design's centre, in document coordinates.
    pub at: Pt,
    /// How much larger than its own size it is placed (1: true to the file).
    #[serde(default = "one")]
    pub scale: f64,
    /// Turned clockwise on the page, in degrees.
    #[serde(default)]
    pub rotation: f64,
}

fn one() -> f64 {
    1.0
}

impl Placement {
    pub fn new(kind: PlacementKind, name: &str, source: &str, width: f64, height: f64, at: Pt) -> Self {
        Self { name: name.trim().to_string(), kind, source: source.to_string(), width, height, at, scale: 1.0, rotation: 0.0 }
    }

    /// Whether every number is usable.
    pub fn is_valid(&self) -> bool {
        let size = |v: f64| v.is_finite() && v > 0.0 && v <= MAX_DESIGN_MM;
        size(self.width)
            && size(self.height)
            && self.at.is_finite()
            && self.scale.is_finite()
            && SCALE_RANGE.contains(&self.scale)
            && self.rotation.is_finite()
            && size(self.width * self.scale)
            && size(self.height * self.scale)
    }

    /// The size on the piece, in mm: the design's own size times the scale.
    pub fn size_mm(&self) -> (f64, f64) {
        (self.width * self.scale, self.height * self.scale)
    }

    /// Scale to a width of `mm` on the piece (the height follows).
    pub fn set_width_mm(&mut self, mm: f64) {
        self.scale = mm / self.width;
    }

    /// Scale to a height of `mm` on the piece (the width follows).
    pub fn set_height_mm(&mut self, mm: f64) {
        self.scale = mm / self.height;
    }

    /// The size as the sewist reads it: `90 x 60 mm`, or `3.54 x 2.36 in` with `inches`.
    pub fn size_text(&self, inches: bool) -> String {
        let (w, h) = self.size_mm();
        if inches {
            format!("{} x {} in", trim(w / MM_PER_IN, 2), trim(h / MM_PER_IN, 2))
        } else {
            format!("{} x {} mm", trim(w, 1), trim(h, 1))
        }
    }

    /// The note for the cut list: `Embroider "Rose" here, 90 x 60 mm`.
    pub fn note(&self) -> String {
        let name = if self.name.is_empty() { "design" } else { self.name.as_str() };
        format!("{} \"{}\" here, {}", self.kind.verb(), name, self.size_text(false))
    }

    /// The point `(x, y)` mm from the design's centre along its own axes (x across, y down), in
    /// document coordinates: turned by the rotation, then moved to the centre.
    pub fn to_page(&self, x_mm: f64, y_mm: f64) -> Pt {
        let (s, c) = self.rotation.to_radians().sin_cos();
        let (x, y) = (mm_to_pt(x_mm), mm_to_pt(y_mm));
        self.at + Pt::new(x * c - y * s, x * s + y * c)
    }

    /// The design's outline: its four corners, clockwise from the top left, in document coordinates.
    pub fn corners(&self) -> [Pt; 4] {
        rectangle(self, self.size_mm())
    }

    /// The crosshair marking the centre: a line across and a line down the design's axes, each
    /// reaching [`CROSSHAIR_OVERSHOOT_MM`] past the design's edge.
    pub fn crosshair(&self) -> [[Pt; 2]; 2] {
        let (w, h) = self.size_mm();
        let (hw, hh) = (w / 2.0 + CROSSHAIR_OVERSHOOT_MM, h / 2.0 + CROSSHAIR_OVERSHOOT_MM);
        [[self.to_page(-hw, 0.0), self.to_page(hw, 0.0)], [self.to_page(0.0, -hh), self.to_page(0.0, hh)]]
    }

    /// What is drawn for the placement, as open polylines in document coordinates: the design's
    /// outline (a ring, its first corner repeated at the end) and the crosshair.
    pub fn marks(&self) -> Vec<Vec<Pt>> {
        let c = self.corners();
        let mut out = vec![vec![c[0], c[1], c[2], c[3], c[0]]];
        out.extend(self.crosshair().into_iter().map(|l| l.to_vec()));
        out
    }
}

/// The corners of a `size` mm rectangle centred on the placement and turned with it.
fn rectangle(p: &Placement, (w, h): (f64, f64)) -> [Pt; 4] {
    let (hw, hh) = (w / 2.0, h / 2.0);
    [p.to_page(-hw, -hh), p.to_page(hw, -hh), p.to_page(hw, hh), p.to_page(-hw, hh)]
}

/// `v` to at most `places` decimals, without trailing zeros.
fn trim(v: f64, places: usize) -> String {
    let s = format!("{v:.places$}");
    if s.contains('.') { s.trim_end_matches('0').trim_end_matches('.').to_string() } else { s }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rose() -> Placement {
        Placement::new(PlacementKind::Embroidery, "Rose", "rose.dst", 90.0, 60.0, Pt::new(100.0, 200.0))
    }

    #[test]
    fn the_size_on_the_piece_is_the_design_size_times_the_scale() {
        let mut p = rose();
        assert_eq!(p.size_mm(), (90.0, 60.0));
        p.scale = 1.5;
        assert_eq!(p.size_mm(), (135.0, 90.0));
        p.set_width_mm(45.0);
        assert_eq!((p.scale, p.size_mm()), (0.5, (45.0, 30.0)));
        p.set_height_mm(120.0);
        assert_eq!(p.size_mm(), (180.0, 120.0));
    }

    #[test]
    fn the_outline_measures_the_size_true_to_scale() {
        // A 100 mm design at scale 1 is exactly 10 cm across: 283.46 pt.
        let p = Placement::new(PlacementKind::Screen, "Square", "sq.png", 100.0, 100.0, Pt::new(0.0, 0.0));
        let c = p.corners();
        assert!(((c[1] - c[0]).len() - 7200.0 / 25.4).abs() < 1e-9);
        assert!(((c[2] - c[1]).len() - 7200.0 / 25.4).abs() < 1e-9);
        // Turned, the sides keep their length and the centre stays.
        let mut r = rose();
        r.rotation = 30.0;
        let c = r.corners();
        assert!(((c[1] - c[0]).len() - mm_to_pt(90.0)).abs() < 1e-9);
        assert!(((c[2] - c[1]).len() - mm_to_pt(60.0)).abs() < 1e-9);
        let mid = (c[0] + c[2]) * 0.5;
        assert!((mid - r.at).len() < 1e-9);
        // A quarter turn clockwise (y down): the top edge becomes the right edge.
        r.rotation = 90.0;
        let c = r.corners();
        assert!((c[0] - (r.at + Pt::new(mm_to_pt(30.0), mm_to_pt(-45.0)))).len() < 1e-9);
    }

    #[test]
    fn the_size_reads_in_mm_or_inches_and_the_note_says_what_to_do() {
        let p = rose();
        assert_eq!(p.size_text(false), "90 x 60 mm");
        assert_eq!(p.size_text(true), "3.54 x 2.36 in");
        assert_eq!(p.note(), "Embroider \"Rose\" here, 90 x 60 mm");
        let s = Placement::new(PlacementKind::Screen, "", "a.svg", 100.5, 50.0, Pt::default());
        assert_eq!(s.note(), "Screen print \"design\" here, 100.5 x 50 mm");
    }

    #[test]
    fn a_crosshair_runs_through_the_centre_past_the_edge() {
        let p = rose();
        let [across, down] = p.crosshair();
        assert!((across[0] - Pt::new(100.0 - mm_to_pt(50.0), 200.0)).len() < 1e-9);
        assert!((down[1] - Pt::new(100.0, 200.0 + mm_to_pt(35.0))).len() < 1e-9);
    }

    #[test]
    fn numbers_out_of_range_are_not_valid() {
        assert!(rose().is_valid());
        for bad in [
            Placement { width: 0.0, ..rose() },
            Placement { height: f64::NAN, ..rose() },
            Placement { scale: 0.0, ..rose() },
            Placement { scale: 101.0, ..rose() },
            Placement { rotation: f64::INFINITY, ..rose() },
            Placement { at: Pt::new(f64::NAN, 0.0), ..rose() },
            Placement { width: 4000.0, scale: 2.0, ..rose() },
        ] {
            assert!(!bad.is_valid(), "{bad:?}");
        }
        let back: Placement = serde_json::from_str(&serde_json::to_string(&rose()).unwrap()).unwrap();
        assert_eq!(back, rose());
    }
}
