//! Designs placed on a pattern piece (PF-12.1, PF-12.2): an embroidery or a screen print, shown
//! at its true size so the sewist sees how big it is and where it sits before cutting.
//!
//! A [`Placement`] keeps the design's own size in mm (read from its file by the host), where its
//! centre sits on the piece in document coordinates, a scale and a turn. The geometry here is in
//! mm; the host converts to its document unit with [`crate::mm_to_pt`].

use serde::{Deserialize, Serialize};

use crate::{MM_PER_PT, Pt};

/// The largest design side, or hoop side, accepted, in mm (a bound for untrusted input).
pub const MAX_DESIGN_MM: f64 = 5000.0;
/// The smallest scale accepted.
pub const MIN_SCALE: f64 = 0.01;
/// The largest scale accepted.
pub const MAX_SCALE: f64 = 100.0;
/// Most designs on one piece.
pub const MAX_PLACEMENTS: usize = 50;

/// What a placed design is, which decides the app that edits it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DesignKind {
    /// Stitches, edited in StitchFoundry; may be shown with a hoop.
    #[default]
    Embroidery,
    /// Artwork or a separation, edited in ScreenFoundry.
    Screen,
}

/// An embroidery hoop's inside, in mm, laid round the design and turning with it.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Hoop {
    pub width: f64,
    pub height: f64,
}

/// A design placed on a piece.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Placement {
    /// What the sewist calls it ("Rose"), shown in the cut-list note.
    pub name: String,
    /// The design's file, as the user gave it.
    #[serde(default)]
    pub file: String,
    #[serde(default)]
    pub kind: DesignKind,
    /// The design's own size at scale 1, in mm.
    pub width: f64,
    pub height: f64,
    /// Where its centre sits, in document coordinates.
    pub at: Pt,
    #[serde(default = "one")]
    pub scale: f64,
    /// Turn in degrees, clockwise on the page (y is down).
    #[serde(default)]
    pub rotation: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hoop: Option<Hoop>,
}

fn one() -> f64 {
    1.0
}

/// Why a placement is refused.
#[derive(Clone, Debug, PartialEq)]
pub enum PlacementError {
    /// A size outside 0 to [`MAX_DESIGN_MM`] (exclusive of 0), or not a number.
    Size(f64),
    /// A scale outside [`MIN_SCALE`] to [`MAX_SCALE`], or not a number.
    Scale(f64),
    /// A position or turn that is not a number.
    NotFinite,
    /// A name longer than [`crate::piece::MAX_NAME`] characters.
    NameTooLong,
    /// More than [`MAX_PLACEMENTS`] designs.
    TooMany,
}

impl std::fmt::Display for PlacementError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PlacementError::Size(v) => write!(f, "design size {v} mm: give more than 0 and at most {MAX_DESIGN_MM} mm"),
            PlacementError::Scale(v) => write!(f, "scale {v}: give {MIN_SCALE} to {MAX_SCALE}"),
            PlacementError::NotFinite => write!(f, "a placement's position and turn must be numbers"),
            PlacementError::NameTooLong => write!(f, "names take at most {} characters", crate::piece::MAX_NAME),
            PlacementError::TooMany => write!(f, "a piece takes at most {MAX_PLACEMENTS} designs"),
        }
    }
}

impl std::error::Error for PlacementError {}

fn check_size(v: f64) -> Result<f64, PlacementError> {
    if v.is_finite() && v > 0.0 && v <= MAX_DESIGN_MM { Ok(v) } else { Err(PlacementError::Size(v)) }
}

impl Hoop {
    pub fn new(width: f64, height: f64) -> Result<Self, PlacementError> {
        Ok(Self { width: check_size(width)?, height: check_size(height)? })
    }
}

impl Placement {
    /// A design of `width` x `height` mm centred at `at`, at scale 1 and not turned.
    pub fn new(name: &str, width: f64, height: f64, at: Pt) -> Result<Self, PlacementError> {
        if name.chars().count() > crate::piece::MAX_NAME {
            return Err(PlacementError::NameTooLong);
        }
        if !at.is_finite() {
            return Err(PlacementError::NotFinite);
        }
        Ok(Self {
            name: name.trim().to_string(),
            file: String::new(),
            kind: DesignKind::default(),
            width: check_size(width)?,
            height: check_size(height)?,
            at,
            scale: 1.0,
            rotation: 0.0,
            hoop: None,
        })
    }

    /// Set the scale.
    pub fn set_scale(&mut self, scale: f64) -> Result<(), PlacementError> {
        if !(scale.is_finite() && (MIN_SCALE..=MAX_SCALE).contains(&scale)) {
            return Err(PlacementError::Scale(scale));
        }
        self.scale = scale;
        Ok(())
    }

    /// Scale so the design is `mm` wide, keeping its shape.
    pub fn set_true_width(&mut self, mm: f64) -> Result<(), PlacementError> {
        self.set_scale(check_size(mm)? / self.width)
    }

    /// Scale so the design is `mm` high, keeping its shape.
    pub fn set_true_height(&mut self, mm: f64) -> Result<(), PlacementError> {
        self.set_scale(check_size(mm)? / self.height)
    }

    /// Move the centre.
    pub fn set_at(&mut self, at: Pt) -> Result<(), PlacementError> {
        if !at.is_finite() {
            return Err(PlacementError::NotFinite);
        }
        self.at = at;
        Ok(())
    }

    /// Turn to `degrees`, kept within 0 up to 360.
    pub fn set_rotation(&mut self, degrees: f64) -> Result<(), PlacementError> {
        if !degrees.is_finite() {
            return Err(PlacementError::NotFinite);
        }
        self.rotation = degrees.rem_euclid(360.0);
        Ok(())
    }

    /// The design's size as placed, in mm (width, height), before the turn.
    pub fn size_mm(&self) -> (f64, f64) {
        (self.width * self.scale, self.height * self.scale)
    }

    /// The corners of a `w` x `h` mm rectangle centred on the design and turned with it, in
    /// document coordinates (in points), clockwise from the top left.
    fn corners_of(&self, w: f64, h: f64) -> [Pt; 4] {
        let (s, c) = self.rotation.to_radians().sin_cos();
        let (hw, hh) = (w / 2.0 / MM_PER_PT, h / 2.0 / MM_PER_PT);
        [(-hw, -hh), (hw, -hh), (hw, hh), (-hw, hh)]
            .map(|(x, y)| Pt::new(self.at.x + x * c - y * s, self.at.y + x * s + y * c))
    }

    /// The outline of the design as placed: its true size, turned about its centre.
    pub fn outline(&self) -> [Pt; 4] {
        let (w, h) = self.size_mm();
        self.corners_of(w, h)
    }

    /// The hoop's outline, turned with the design, if there is a hoop.
    pub fn hoop_outline(&self) -> Option<[Pt; 4]> {
        self.hoop.map(|h| self.corners_of(h.width, h.height))
    }

    /// Whether the design (as placed, scaled) lies inside the hoop, both sides turned alike.
    pub fn fits_hoop(&self) -> Option<bool> {
        let (w, h) = self.size_mm();
        self.hoop.map(|hp| w <= hp.width + 1e-9 && h <= hp.height + 1e-9)
    }

    /// The cut-list note: `Embroider "Rose" here, 90 x 60 mm`.
    pub fn note(&self) -> String {
        let (w, h) = self.size_mm();
        let verb = match self.kind {
            DesignKind::Embroidery => "Embroider",
            DesignKind::Screen => "Print",
        };
        format!("{verb} \"{}\" here, {} x {} mm", self.name, round1(w), round1(h))
    }
}

/// `v` rounded to 0.1 mm, without a trailing ".0".
fn round1(v: f64) -> String {
    let r = (v * 10.0).round() / 10.0;
    if r.fract() == 0.0 { format!("{}", r as i64) } else { format!("{r:.1}") }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rose() -> Placement {
        Placement::new("Rose", 90.0, 60.0, Pt::new(100.0, 200.0)).unwrap()
    }

    fn dist(a: Pt, b: Pt) -> f64 {
        (a - b).len() * MM_PER_PT
    }

    #[test]
    fn outline_is_true_size_in_mm() {
        let o = rose().outline();
        assert!((dist(o[0], o[1]) - 90.0).abs() < 1e-9);
        assert!((dist(o[1], o[2]) - 60.0).abs() < 1e-9);
    }

    #[test]
    fn scale_and_true_width_resize_the_outline() {
        let mut p = rose();
        p.set_scale(2.0).unwrap();
        assert_eq!(p.size_mm(), (180.0, 120.0));
        p.set_true_width(45.0).unwrap();
        assert!((p.scale - 0.5).abs() < 1e-12);
        p.set_true_height(30.0).unwrap();
        let o = p.outline();
        assert!((dist(o[0], o[1]) - 45.0).abs() < 1e-9);
    }

    #[test]
    fn turning_keeps_the_size_and_centre() {
        let mut p = rose();
        p.set_rotation(90.0).unwrap();
        let o = p.outline();
        assert!((dist(o[0], o[1]) - 90.0).abs() < 1e-9);
        let mid = (o[0] + o[2]) * 0.5;
        assert!((mid - p.at).len() < 1e-9);
        // A quarter turn puts the top edge going down the page.
        assert!((o[1].x - o[0].x).abs() < 1e-9 && o[1].y > o[0].y);
        p.set_rotation(-90.0).unwrap();
        assert_eq!(p.rotation, 270.0);
    }

    #[test]
    fn hoop_turns_with_the_design_and_says_if_it_fits() {
        let mut p = rose();
        assert_eq!(p.fits_hoop(), None);
        p.hoop = Some(Hoop::new(100.0, 100.0).unwrap());
        assert_eq!(p.fits_hoop(), Some(true));
        let h = p.hoop_outline().unwrap();
        assert!((dist(h[0], h[1]) - 100.0).abs() < 1e-9);
        p.set_scale(1.2).unwrap();
        assert_eq!(p.fits_hoop(), Some(false));
    }

    #[test]
    fn note_names_the_design_and_its_size() {
        assert_eq!(rose().note(), "Embroider \"Rose\" here, 90 x 60 mm");
        let mut p = rose();
        p.kind = DesignKind::Screen;
        p.set_scale(0.55).unwrap();
        assert_eq!(p.note(), "Print \"Rose\" here, 49.5 x 33 mm");
    }

    #[test]
    fn refuses_bad_values() {
        let at = Pt::new(0.0, 0.0);
        assert_eq!(Placement::new("x", 0.0, 10.0, at), Err(PlacementError::Size(0.0)));
        assert!(Placement::new("x", f64::NAN, 10.0, at).is_err());
        assert!(Placement::new("x", 10.0, MAX_DESIGN_MM + 1.0, at).is_err());
        assert_eq!(Placement::new("x", 1.0, 1.0, Pt::new(f64::NAN, 0.0)), Err(PlacementError::NotFinite));
        let mut p = rose();
        assert_eq!(p.set_scale(0.0), Err(PlacementError::Scale(0.0)));
        assert!(p.set_scale(f64::INFINITY).is_err());
        assert!(p.set_rotation(f64::NAN).is_err());
        assert!(Hoop::new(-1.0, 5.0).is_err());
    }
}
