//! The data a pattern piece carries.
//!
//! A piece is drawn by its **seam line** (where it is sewn), a closed path. Its edges are the
//! path's segments, numbered from 0 in drawing order: edge `i` runs from anchor `i` to anchor
//! `i + 1`. The seam allowance, notches and corner styles are kept per edge, so they follow the
//! seam line when its points are moved.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::Pt;

/// Largest seam allowance accepted, in mm (wider than any hem; a bound for untrusted input).
pub const MAX_ALLOWANCE_MM: f64 = 200.0;
/// Most copies of a piece to cut.
pub const MAX_CUT: u32 = 999;
/// Most notches on one piece.
pub const MAX_NOTCHES: usize = 500;
/// Longest piece name or fabric name, in characters.
pub const MAX_NAME: usize = 200;
/// The widest drill hole, in mm.
pub const MAX_DRILL_MM: f64 = 50.0;

/// The default seam allowance, 1 cm (common in sewing patterns sold in Europe; 5/8 in, 15.875 mm,
/// is the common one in North America and a preference away).
pub const DEFAULT_ALLOWANCE_MM: f64 = 10.0;

/// How the cutting line turns at a corner between two edges.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Corner {
    /// The two allowance lines meet where they cross (a sharp corner).
    #[default]
    Mitre,
    /// The corner is cut square: at a corner sharper than a right angle each allowance line stops
    /// at the other edge's allowance width past the corner and the two ends are joined, so a
    /// pointed corner gets no long spike of allowance. At a right angle or wider it is a mitre.
    Square,
}

/// The kind of mark a notch makes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum NotchKind {
    /// A single slit, cut in from the cutting line.
    #[default]
    Slit,
    /// Two slits side by side (often the back of a garment).
    Double,
    /// A V-shaped notch in the allowance.
    V,
    /// A slit with a bar across its end (PF-7.7).
    T,
    /// A square-ended cut of even width (a castle notch).
    Castle,
    /// A cut of even width with a round end.
    U,
}

/// A notch: a mark on an edge, matched with the notch on the piece it is sewn to.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Notch {
    /// The edge it is on.
    pub edge: usize,
    /// Where on the edge, from its start (0) to its end (1), by length along the seam line.
    pub at: f64,
    #[serde(default)]
    pub kind: NotchKind,
}

/// The grainline: the arrow laid along the fabric's lengthwise grain, in document coordinates.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Grainline {
    pub from: Pt,
    pub to: Pt,
}

/// A drill hole (PF-7.6): a punch mark through the piece, for a dart's point, a pocket's corners
/// or a button, in document coordinates.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Drill {
    pub at: Pt,
    /// The punch's diameter, in mm; 0 for a mark only.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub diameter: f64,
}

fn is_zero(v: &f64) -> bool {
    *v == 0.0
}

/// A pattern piece.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Piece {
    /// What it is called (Front Bodice, Sleeve…), shown in its label.
    #[serde(default)]
    pub name: String,
    /// How many to cut.
    #[serde(default = "one")]
    pub cut: u32,
    /// Cut in mirror-image pairs (half of `cut` the other way up, as when the fabric is folded).
    #[serde(default)]
    pub mirror: bool,
    /// The fabric it is cut from (Main, Lining, Interfacing…).
    #[serde(default)]
    pub fabric: String,
    /// The seam allowance of every edge without its own, in mm.
    #[serde(default = "default_allowance")]
    pub allowance: f64,
    /// Edges with their own allowance, in mm (0 for an edge cut on the seam line, as a fold).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub edges: BTreeMap<usize, f64>,
    /// Corners that are not mitred: corner `i` is where edge `i - 1` meets edge `i` (corner 0
    /// joins the last edge to the first).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub corners: BTreeMap<usize, Corner>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notches: Vec<Notch>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grainline: Option<Grainline>,
    /// Grade rules on the seam line's points (PF-6.2).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub grade: Vec<crate::grade::GradeRule>,
    /// The edge cut on the fold (PF-9.2): laid against the fabric's fold, it takes no allowance,
    /// and the piece cut is the whole, opened out across it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fold: Option<usize>,
    /// Drill holes (PF-7.6), in the order they were added.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub drills: Vec<Drill>,
    /// Runs of the seam line sewn to another piece (PF-10.2), each with the label the run it is
    /// sewn to has on that piece.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sewn: Vec<Sewn>,
    /// Embroidery and screen-print designs placed on the piece (PF-12), in the order placed.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub placements: Vec<crate::placement::Placement>,
}

/// A run of a piece's seam line sewn to a run of another (or the same) piece's: from anchor
/// `from` forward to anchor `to`, sewn to the run with the same `label` ("A", "B"…).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Sewn {
    pub label: String,
    pub from: usize,
    pub to: usize,
}

fn one() -> u32 {
    1
}

fn default_allowance() -> f64 {
    DEFAULT_ALLOWANCE_MM
}

impl Default for Piece {
    fn default() -> Self {
        Self {
            name: String::new(),
            cut: 1,
            mirror: false,
            fabric: String::new(),
            allowance: DEFAULT_ALLOWANCE_MM,
            edges: BTreeMap::new(),
            corners: BTreeMap::new(),
            notches: Vec::new(),
            grainline: None,
            grade: Vec::new(),
            fold: None,
            drills: Vec::new(),
            sewn: Vec::new(),
            placements: Vec::new(),
        }
    }
}

/// Why a value can't go into a piece.
#[derive(Clone, Debug, PartialEq)]
pub enum PieceError {
    /// An allowance outside 0 to [`MAX_ALLOWANCE_MM`], or not a number.
    Allowance(f64),
    /// A cut count outside 1 to [`MAX_CUT`].
    Cut(u32),
    /// A notch position outside 0 to 1, or not a number.
    NotchAt(f64),
    /// More than [`MAX_NOTCHES`] notches.
    TooManyNotches,
    /// A name longer than [`MAX_NAME`] characters.
    NameTooLong,
    /// A drill hole off the page (not a number) or a diameter outside 0 to [`MAX_DRILL_MM`].
    Drill(f64),
    /// More than [`MAX_NOTCHES`] drill holes.
    TooManyDrills,
    /// A design that is not a usable size, scale or place.
    Placement,
    /// More than [`crate::placement::MAX_PLACEMENTS`] designs.
    TooManyPlacements,
}

impl std::fmt::Display for PieceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PieceError::Allowance(v) => {
                write!(f, "seam allowance {v} mm: give 0 to {MAX_ALLOWANCE_MM} mm")
            }
            PieceError::Cut(v) => write!(f, "cut {v}: give 1 to {MAX_CUT}"),
            PieceError::NotchAt(v) => write!(f, "notch position {v}: give 0 (the edge's start) to 1 (its end)"),
            PieceError::TooManyNotches => write!(f, "a piece takes at most {MAX_NOTCHES} notches"),
            PieceError::NameTooLong => write!(f, "names take at most {MAX_NAME} characters"),
            PieceError::Drill(v) => write!(f, "drill hole {v}: give a point and a diameter of 0 to {MAX_DRILL_MM} mm"),
            PieceError::TooManyDrills => write!(f, "a piece takes at most {MAX_NOTCHES} drill holes"),
            PieceError::Placement => write!(
                f,
                "a design needs a size of 0 to {} mm, a scale of 0.01 to 100 and a place on the page",
                crate::placement::MAX_DESIGN_MM
            ),
            PieceError::TooManyPlacements => write!(f, "a piece takes at most {} designs", crate::placement::MAX_PLACEMENTS),
        }
    }
}

impl std::error::Error for PieceError {}

/// An allowance in mm, checked.
pub fn check_allowance(mm: f64) -> Result<f64, PieceError> {
    if mm.is_finite() && (0.0..=MAX_ALLOWANCE_MM).contains(&mm) { Ok(mm) } else { Err(PieceError::Allowance(mm)) }
}

/// A name, checked.
pub fn check_name(s: &str) -> Result<String, PieceError> {
    if s.chars().count() <= MAX_NAME { Ok(s.trim().to_string()) } else { Err(PieceError::NameTooLong) }
}

impl Piece {
    /// The allowance of `edge`, in mm (none on the fold).
    pub fn allowance_of(&self, edge: usize) -> f64 {
        if self.fold == Some(edge) {
            return 0.0;
        }
        self.edges.get(&edge).copied().unwrap_or(self.allowance)
    }

    /// The style of corner `i` (where edge `i - 1` meets edge `i`).
    pub fn corner_of(&self, i: usize) -> Corner {
        self.corners.get(&i).copied().unwrap_or_default()
    }

    pub fn set_cut(&mut self, cut: u32) -> Result<(), PieceError> {
        if (1..=MAX_CUT).contains(&cut) {
            self.cut = cut;
            Ok(())
        } else {
            Err(PieceError::Cut(cut))
        }
    }

    /// Add a notch, kept in order along the seam line.
    pub fn add_notch(&mut self, n: Notch) -> Result<(), PieceError> {
        if !(n.at.is_finite() && (0.0..=1.0).contains(&n.at)) {
            return Err(PieceError::NotchAt(n.at));
        }
        if self.notches.len() >= MAX_NOTCHES {
            return Err(PieceError::TooManyNotches);
        }
        self.notches.push(n);
        self.notches.sort_by(|a, b| a.edge.cmp(&b.edge).then(a.at.total_cmp(&b.at)));
        Ok(())
    }

    /// Add a drill hole.
    pub fn add_drill(&mut self, d: Drill) -> Result<(), PieceError> {
        if !(d.at.is_finite()) {
            return Err(PieceError::Drill(f64::NAN));
        }
        if !(d.diameter.is_finite() && (0.0..=MAX_DRILL_MM).contains(&d.diameter)) {
            return Err(PieceError::Drill(d.diameter));
        }
        if self.drills.len() >= MAX_NOTCHES {
            return Err(PieceError::TooManyDrills);
        }
        self.drills.push(d);
        Ok(())
    }

    /// Add a design placed on the piece.
    pub fn add_placement(&mut self, p: crate::placement::Placement) -> Result<(), PieceError> {
        if !p.is_valid() {
            return Err(PieceError::Placement);
        }
        if self.placements.len() >= crate::placement::MAX_PLACEMENTS {
            return Err(PieceError::TooManyPlacements);
        }
        self.placements.push(p);
        Ok(())
    }

    /// Forget what refers to edges a seam line of `edges` edges doesn't have (after points were
    /// deleted): their allowances, corners and notches.
    pub fn fit_edges(&mut self, edges: usize) {
        self.edges.retain(|e, _| *e < edges);
        self.corners.retain(|c, _| *c < edges);
        self.notches.retain(|n| n.edge < edges);
        self.fold = self.fold.filter(|f| *f < edges);
        self.sewn.retain(|r| r.from < edges && r.to < edges);
    }

    /// The label text: the name, then how to cut it.
    pub fn label(&self) -> String {
        let name = if self.name.is_empty() { "Piece" } else { self.name.as_str() };
        let mut cut = format!("Cut {}", self.cut);
        if self.fold.is_some() {
            cut.push_str(" on fold");
        }
        if self.mirror {
            cut.push_str(" (mirrored pairs)");
        }
        if !self.fabric.is_empty() {
            cut.push_str(" · ");
            cut.push_str(&self.fabric);
        }
        format!("{name}\n{cut}")
    }

    /// Move the positions the piece stores (its grainline and drill holes) by `f`, as the host
    /// transforms the piece.
    pub fn transform(&mut self, f: impl Fn(Pt) -> Pt) {
        if let Some(g) = &mut self.grainline {
            g.from = f(g.from);
            g.to = f(g.to);
        }
        for d in &mut self.drills {
            d.at = f(d.at);
        }
        for p in &mut self.placements {
            p.at = f(p.at);
        }
    }
}

/// The part of a pattern piece a node of the host document is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Mark {
    /// The seam line: the path the piece is drawn and edited by.
    Seam,
    /// The cutting line, generated.
    Cut,
    /// The notches, generated.
    Notches,
    /// The grainline arrow, generated.
    Grainline,
    /// The label, generated.
    Label,
    /// The drill holes, generated.
    Drills,
    /// The outlines and crosshairs of the designs placed on the piece, generated.
    Placements,
}

/// What the host keeps on a node of a pattern: the piece (on the group holding it) or which part
/// of a piece the node draws.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PatternTag {
    Piece(Box<Piece>),
    Mark(Mark),
    /// On an image: a photo of a fabric (PF-10.16).
    Fabric(FabricPhoto),
}

/// A photo of a fabric (PF-10.16), for seeing the draped garment in it: the fabric pieces name
/// (`Piece::fabric`), how wide the photo is on the fabric (mm), and how many quarter turns
/// clockwise bring the fabric's lengthwise grain (the selvedge's direction) upright in it; and
/// whether its light is evened out and it is made to repeat without a seam (PF-10.17,
/// [`crate::swatch`]).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FabricPhoto {
    #[serde(default)]
    pub name: String,
    pub width: f64,
    #[serde(default)]
    pub turns: u8,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub even: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub seamless: bool,
}

/// The narrowest and widest a fabric photo can be, mm.
pub const FABRIC_PHOTO_WIDTH: std::ops::RangeInclusive<f64> = 10.0..=5000.0;

impl PatternTag {
    pub fn piece(&self) -> Option<&Piece> {
        match self {
            PatternTag::Piece(p) => Some(p),
            _ => None,
        }
    }
    pub fn mark(&self) -> Option<Mark> {
        match self {
            PatternTag::Mark(m) => Some(*m),
            _ => None,
        }
    }
    pub fn fabric(&self) -> Option<&FabricPhoto> {
        match self {
            PatternTag::Fabric(f) => Some(f),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_piece_round_trips_and_an_empty_one_fills_in_its_defaults() {
        let mut p = Piece { name: "Front".into(), fabric: "Main".into(), ..Piece::default() };
        p.edges.insert(2, 0.0);
        p.corners.insert(1, Corner::Square);
        p.add_notch(Notch { edge: 1, at: 0.5, kind: NotchKind::Double }).unwrap();
        p.grainline = Some(Grainline { from: Pt::new(1.0, 2.0), to: Pt::new(1.0, 50.0) });
        p.add_drill(Drill { at: Pt::new(20.0, 30.0), diameter: 0.0 }).unwrap();
        let tag = PatternTag::Piece(Box::new(p.clone()));
        let back: PatternTag = serde_json::from_str(&serde_json::to_string(&tag).unwrap()).unwrap();
        assert_eq!(back, tag);
        let empty: Piece = serde_json::from_str("{}").unwrap();
        assert_eq!(empty, Piece::default());
        assert_eq!((empty.cut, empty.allowance), (1, 10.0));
        assert_eq!(p.allowance_of(2), 0.0);
        assert_eq!(p.allowance_of(0), 10.0);
        assert_eq!(p.corner_of(1), Corner::Square);
        assert_eq!(p.corner_of(0), Corner::Mitre);
    }

    #[test]
    fn values_out_of_range_are_refused() {
        let mut p = Piece::default();
        assert!(check_allowance(-1.0).is_err() && check_allowance(f64::NAN).is_err() && check_allowance(201.0).is_err());
        assert_eq!(check_allowance(15.875), Ok(15.875));
        assert!(p.set_cut(0).is_err() && p.set_cut(1000).is_err());
        assert!(p.add_notch(Notch { edge: 0, at: 1.5, kind: NotchKind::Slit }).is_err());
        assert!(p.add_notch(Notch { edge: 0, at: f64::NAN, kind: NotchKind::Slit }).is_err());
        assert!(check_name(&"x".repeat(201)).is_err());
        assert!(p.add_drill(Drill { at: Pt::new(f64::NAN, 0.0), diameter: 0.0 }).is_err());
        assert!(p.add_drill(Drill { at: Pt::new(0.0, 0.0), diameter: 51.0 }).is_err());
        assert!(p.add_drill(Drill { at: Pt::new(0.0, 0.0), diameter: -1.0 }).is_err());
        assert_eq!(p.add_drill(Drill { at: Pt::new(3.0, 4.0), diameter: 3.0 }), Ok(()));
        p.transform(|q| q + Pt::new(1.0, 1.0));
        assert_eq!(p.drills, [Drill { at: Pt::new(4.0, 5.0), diameter: 3.0 }]);
        assert_eq!(check_name("  Sleeve "), Ok("Sleeve".into()));
    }

    #[test]
    fn notches_stay_in_order_and_go_with_their_edges() {
        let mut p = Piece::default();
        for (edge, at) in [(2, 0.5), (0, 0.7), (0, 0.2)] {
            p.add_notch(Notch { edge, at, kind: NotchKind::Slit }).unwrap();
        }
        let order: Vec<(usize, f64)> = p.notches.iter().map(|n| (n.edge, n.at)).collect();
        assert_eq!(order, [(0, 0.2), (0, 0.7), (2, 0.5)]);
        p.edges.insert(2, 5.0);
        p.fit_edges(2);
        assert_eq!(p.notches.len(), 2);
        assert!(p.edges.is_empty());
    }

    #[test]
    fn the_label_says_how_to_cut() {
        let p = Piece { name: "Sleeve".into(), cut: 2, mirror: true, fabric: "Main".into(), ..Piece::default() };
        assert_eq!(p.label(), "Sleeve\nCut 2 (mirrored pairs) · Main");
        assert_eq!(Piece::default().label(), "Piece\nCut 1");
        let mut p = Piece { name: "Front".into(), fold: Some(3), ..Piece::default() };
        assert_eq!(p.label(), "Front\nCut 1 on fold");
        // The fold takes no allowance, and goes when its edge does.
        p.edges.insert(3, 15.0);
        assert_eq!((p.allowance_of(3), p.allowance_of(2)), (0.0, 10.0));
        p.fit_edges(3);
        assert_eq!(p.fold, None);
    }
}
