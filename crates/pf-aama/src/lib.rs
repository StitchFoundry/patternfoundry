//! Pattern pieces as DXF-AAMA / ASTM D6673 files, the format cutting rooms, grading and nesting
//! systems read, written and read back on the DXF tags of [`pf_dxf`]. From the published
//! descriptions of the standard: DH Patterns and Fit's technical overview
//! (<https://dorthehansen.com/wp-content/uploads/2014/10/ePattern-ASTM-Standard.pdf>) and Patro's
//! file format notes (<https://fabricesalvaire.github.io/Patro/resources/file-format/dxf-astm.html>).
//!
//! One block per piece, inserted at the origin, its entities on layers named by number: 1 the
//! boundary (the cutting line) and the piece system text, 2 turn points and 3 curve points (of
//! layers 1 and 14), 4 slit and V notches (points carrying their depth, width and angle), 80 T
//! notches (the point at the notch's depth), 81 castle notches and 83 U notches, 6 the mirror line (the fold a
//! piece is cut on, "symmetry of fold" in Patro's notes), 7 the grain line,
//! 8 internal lines, 13 drill holes (points whose z is the punch's diameter, as DH Patterns and
//! Fit's overview describes them), 14 the sew line, and 84 and 87 the quality validation curves (copies of the
//! boundary and sew line). The style system text is on layer 1 at the origin, outside every
//! block. Coordinates are millimetres (`Units: METRIC`), y up.

use pf_dxf::{Tag, Value};

/// A point in millimetres, y up.
pub type Mm = [f64; 2];

/// The DXF version of the file.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Version {
    /// R12 (AC1009), for older AAMA readers.
    R12,
    /// R13 (AC1012), which ASTM D6673 is based on.
    #[default]
    R13,
}

impl Version {
    pub fn acadver(self) -> &'static str {
        match self {
            Version::R12 => "AC1009",
            Version::R13 => "AC1012",
        }
    }

    /// From R13 objects carry handles and subclass markers.
    fn handles(self) -> bool {
        self == Version::R13
    }
}

/// The shape of a notch, which the layer of its point gives.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum NotchShape {
    /// A slit, or a V when it has a width (layer 4).
    #[default]
    Plain,
    /// A slit with a bar of the width across its end (layer 80).
    T,
    /// A cut of even width with a square end (layer 81).
    Castle,
    /// A cut of even width with a semicircle reaching the depth (layer 83).
    U,
}

impl NotchShape {
    fn layer(self) -> &'static str {
        match self {
            NotchShape::Plain => "4",
            NotchShape::T => "80",
            NotchShape::Castle => "81",
            NotchShape::U => "83",
        }
    }
}

/// A notch as ASTM describes it with a point on layer 4, 80, 81 or 83.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Notch {
    /// On the boundary (the middle of a V notch's opening).
    pub at: Mm,
    /// The direction it is cut, into the piece: degrees counter-clockwise from +x.
    pub angle: f64,
    pub depth: f64,
    /// The opening's width at the boundary: 0 for a slit, more for a V notch; a T notch's bar, a
    /// castle or U notch's cut.
    pub width: f64,
    pub shape: NotchShape,
}

impl Notch {
    /// The notch's cut as polylines. Its angle is counter-clockwise from +x with y up; with
    /// `y_down` the points are in a y-down space (as a document's) and the angle turned to match.
    pub fn lines(&self, y_down: bool) -> Vec<Vec<Mm>> {
        let (s, c) = self.angle.to_radians().sin_cos();
        let into = if y_down { [c, -s] } else { [c, s] };
        let side = [-into[1], into[0]];
        let p = |along: f64, across: f64| [self.at[0] + into[0] * along + side[0] * across, self.at[1] + into[1] * along + side[1] * across];
        let (d, h) = (self.depth, self.width / 2.0);
        match self.shape {
            NotchShape::Plain if h > 0.0 => vec![vec![p(0.0, h), p(d, 0.0), p(0.0, -h)]],
            NotchShape::Plain => vec![vec![p(0.0, 0.0), p(d, 0.0)]],
            NotchShape::T => vec![vec![p(0.0, 0.0), p(d, 0.0)], vec![p(d, h), p(d, -h)]],
            NotchShape::Castle => vec![vec![p(0.0, h), p(d, h), p(d, -h), p(0.0, -h)]],
            NotchShape::U => {
                let r = h.min(d);
                let mut line = vec![p(0.0, h)];
                const STEPS: usize = 8;
                for i in 0..=STEPS {
                    let (s, k) = (i as f64 * std::f64::consts::PI / STEPS as f64).sin_cos();
                    line.push(p(d - r + r * s, h * k));
                }
                line.push(p(0.0, -h));
                vec![line]
            }
        }
    }
}

/// A drill hole: a point on layer 13.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Drill {
    pub at: Mm,
    /// The punch's diameter (the point's z); 0 for none given.
    pub diameter: f64,
}

/// A pattern piece.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Piece {
    /// Its name (unique in the file).
    pub name: String,
    /// How many to cut: right-side pieces and left-side (mirrored) pieces.
    pub quantity: (u32, u32),
    /// The material it is cut from; empty for none.
    pub material: String,
    /// The cutting line, closed (first point not repeated).
    pub boundary: Vec<Mm>,
    /// The sew line, closed; empty for none (the boundary is then the sew line too).
    pub sew_line: Vec<Mm>,
    pub notches: Vec<Notch>,
    pub grain: Option<[Mm; 2]>,
    /// The mirror line: the fold the piece is cut on, the piece being one half of what is cut
    /// (layer 6).
    pub mirror: Option<[Mm; 2]>,
    /// Lines drawn on the piece but not cut (placements, fold lines, darts).
    pub internal: Vec<Vec<Mm>>,
    pub drills: Vec<Drill>,
    /// Notes for the cut list (a design to embroider here, 90 x 60 mm): shown on the projector
    /// view and the cutting SVG's labels; not written to the DXF-AAMA file.
    pub notes: Vec<String>,
    /// Where the piece system text starts (inside the piece).
    pub text_at: Mm,
}

/// A pattern style: the pieces and what the style system text says about them.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Style {
    pub name: String,
    /// `Author:`, as vendor;application;release.
    pub author: String,
    /// `dd-mm-yyyy`.
    pub date: String,
    /// `hh:mm`.
    pub time: String,
    pub sample_size: String,
    /// How far, in mm, curves may stray from the points written (`Curve Tolerance:`).
    pub tolerance: f64,
    pub version: Version,
    pub pieces: Vec<Piece>,
}

/// Why a style can't be written, or a file read.
#[derive(Clone, Debug, PartialEq)]
pub enum AamaError {
    NoPieces,
    NoOutline(String),
    /// The DXF tags couldn't be read.
    Dxf(String),
}

impl std::fmt::Display for AamaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AamaError::NoPieces => f.write_str("no pattern pieces to export"),
            AamaError::NoOutline(name) => write!(f, "the piece {name:?} has no outline"),
            AamaError::Dxf(e) => write!(f, "not a readable DXF file: {e}"),
        }
    }
}

impl std::error::Error for AamaError {}

/// Text height of the system text, mm.
const TEXT_MM: f64 = 3.5;
/// A turn of the outline sharper than this (degrees) is a turn point; a gentler one a curve point.
const TURN_DEGREES: f64 = 15.0;
/// Turns gentler than this are on a straight line: neither.
const STRAIGHT_DEGREES: f64 = 0.01;
/// The layers a file has (layer 0 is not used).
const LAYERS: [&str; 15] = ["1", "2", "3", "4", "6", "7", "8", "13", "14", "15", "80", "81", "83", "84", "87"];

/// The handles of the objects every R13 drawing has; entities and table entries count up from
/// [`FIRST_FREE`].
mod fixed {
    pub const LAYER_TABLE: u32 = 0x1;
    pub const LTYPE_TABLE: u32 = 0x2;
    pub const APPID_TABLE: u32 = 0x3;
    pub const DIMSTYLE_TABLE: u32 = 0x4;
    pub const STYLE_TABLE: u32 = 0x5;
    pub const UCS_TABLE: u32 = 0x6;
    pub const VIEW_TABLE: u32 = 0x7;
    pub const VPORT_TABLE: u32 = 0x8;
    pub const BLOCK_RECORD_TABLE: u32 = 0x9;
    pub const ROOT: u32 = 0xA;
    pub const GROUPS: u32 = 0xB;
    pub const MODEL_RECORD: u32 = 0xD;
    pub const PAPER_RECORD: u32 = 0xE;
    pub const MODEL_BLOCK: u32 = 0xF;
    pub const MODEL_END: u32 = 0x10;
    pub const PAPER_BLOCK: u32 = 0x11;
    pub const PAPER_END: u32 = 0x12;
    pub const ACTIVE_VPORT: u32 = 0x15;
    pub const ACAD_APPID: u32 = 0x16;
    pub const STANDARD_DIMSTYLE: u32 = 0x17;
    pub const FIRST_FREE: u32 = 0x30;
}

/// The turn points and curve points of a closed polyline.
pub fn turn_and_curve_points(poly: &[Mm]) -> (Vec<Mm>, Vec<Mm>) {
    let n = poly.len();
    let (mut turns, mut curves) = (Vec::new(), Vec::new());
    if n < 3 {
        return (poly.to_vec(), curves);
    }
    for i in 0..n {
        let (a, b, c) = (poly[(i + n - 1) % n], poly[i], poly[(i + 1) % n]);
        let (u, v) = ([b[0] - a[0], b[1] - a[1]], [c[0] - b[0], c[1] - b[1]]);
        if u == [0.0, 0.0] || v == [0.0, 0.0] {
            continue;
        }
        let turn = (u[0] * v[1] - u[1] * v[0]).atan2(u[0] * v[0] + u[1] * v[1]).to_degrees().abs();
        if turn >= TURN_DEGREES {
            turns.push(b);
        } else if turn > STRAIGHT_DEGREES {
            curves.push(b);
        }
    }
    (turns, curves)
}

/// A block name from a piece name: up to 31 capitals, digits, `$`, `-` and `_` (what R12 and
/// R13 allow), unique among `taken` (`_2`, `_3`…).
fn block_name(piece: &str, taken: &[String]) -> String {
    let base: String =
        piece.trim().chars().map(|c| if c.is_ascii_alphanumeric() || "$-_".contains(c) { c.to_ascii_uppercase() } else { '_' }).take(31).collect();
    let base = if base.trim_matches('_').is_empty() { "PIECE".to_string() } else { base };
    let free = |n: &str| !taken.iter().any(|t| t.eq_ignore_ascii_case(n));
    if free(&base) {
        return base;
    }
    (2..)
        .map(|i| {
            let suffix = format!("_{i}");
            format!("{}{suffix}", base.chars().take(31 - suffix.len()).collect::<String>())
        })
        .find(|n| free(n))
        .unwrap_or(base)
}

/// Tags being written, with the next free handle.
struct Out {
    tags: Vec<Tag>,
    handles: bool,
    next: u32,
}

impl Out {
    fn s(&mut self, code: i32, v: impl Into<String>) {
        self.tags.push(Tag::s(code, v));
    }
    fn f(&mut self, code: i32, v: f64) {
        self.tags.push(Tag::f(code, v));
    }
    fn i(&mut self, code: i32, v: i64) {
        self.tags.push(Tag::i(code, v));
    }
    fn xyz(&mut self, code: i32, x: f64, y: f64, z: f64) {
        self.f(code, x);
        self.f(code + 10, y);
        self.f(code + 20, z);
    }
    fn handle(&mut self) -> u32 {
        let h = self.next;
        self.next += 1;
        h
    }
    fn hex(&mut self, code: i32, h: u32) {
        self.s(code, format!("{h:X}"));
    }
    /// An object's start: its type, and from R13 its handle, owner and first subclass.
    fn object(&mut self, kind: &str, h: u32, owner: u32, subclass: &str) {
        self.s(0, kind);
        if self.handles {
            self.hex(if kind == "DIMSTYLE" { 105 } else { 5 }, h);
            self.hex(330, owner);
            self.s(100, subclass);
        }
    }
    /// An entity's start on `layer`, owned by `owner` (a block record, or a polyline for its
    /// vertices), ending with its own subclass (none when empty).
    fn entity(&mut self, kind: &str, owner: u32, layer: &str, subclass: &str) {
        let h = self.handle();
        self.object(kind, h, owner, "AcDbEntity");
        self.s(8, layer);
        self.i(62, 7);
        if self.handles && !subclass.is_empty() {
            self.s(100, subclass);
        }
    }
    fn text(&mut self, owner: u32, at: Mm, s: &str) {
        self.entity("TEXT", owner, "1", "AcDbText");
        self.xyz(10, at[0], at[1], 0.0);
        self.f(40, TEXT_MM);
        self.s(1, s);
        if self.handles {
            self.s(100, "AcDbText");
        }
    }
    fn polyline(&mut self, owner: u32, layer: &str, pts: &[Mm], closed: bool) {
        let h = self.next;
        self.entity("POLYLINE", owner, layer, "AcDb2dPolyline");
        self.i(66, 1);
        self.xyz(10, 0.0, 0.0, 0.0);
        self.i(70, i64::from(closed));
        for p in pts {
            self.entity("VERTEX", h, layer, "AcDbVertex");
            if self.handles {
                self.s(100, "AcDb2dVertex");
            }
            self.xyz(10, p[0], p[1], 0.0);
        }
        self.entity("SEQEND", h, layer, "");
    }
    fn point(&mut self, owner: u32, layer: &str, p: Mm) {
        self.entity("POINT", owner, layer, "AcDbPoint");
        self.xyz(10, p[0], p[1], 0.0);
    }
    fn table(&mut self, name: &str, h: u32, count: usize) {
        self.s(0, "TABLE");
        self.s(2, name);
        if self.handles {
            self.hex(5, h);
            self.s(330, "0");
            self.s(100, "AcDbSymbolTable");
        }
        self.i(70, count as i64);
    }
    /// A table entry: the shared record subclass, then the table's own (from R13).
    fn record(&mut self, kind: &str, h: u32, table: u32, subclass: &str) {
        self.object(kind, h, table, "AcDbSymbolTableRecord");
        if self.handles {
            self.s(100, subclass);
        }
    }
}

/// The style as a DXF-AAMA file (ASCII DXF).
pub fn write(style: &Style) -> Result<String, AamaError> {
    if style.pieces.is_empty() {
        return Err(AamaError::NoPieces);
    }
    let v = style.version;
    let mut o = Out { tags: Vec::new(), handles: v.handles(), next: fixed::FIRST_FREE };
    let mut extents = [0.0, 0.0, 0.0, 0.0f64];
    // Each piece's entities, in a block of its own: (name, record, tags).
    let mut blocks: Vec<(String, u32, Vec<Tag>)> = Vec::new();
    for piece in &style.pieces {
        if piece.boundary.len() < 3 {
            return Err(AamaError::NoOutline(piece.name.clone()));
        }
        let taken: Vec<String> = blocks.iter().map(|b| b.0.clone()).collect();
        let name = block_name(&piece.name, &taken);
        let record = o.handle();
        let start = o.tags.len();
        let mut lines = vec![format!("Piece Name: {}", piece.name), format!("Quantity: {},{}", piece.quantity.0, piece.quantity.1)];
        if !piece.material.is_empty() {
            lines.push(format!("Material: {}", piece.material));
        }
        for (i, l) in lines.iter().enumerate() {
            o.text(record, [piece.text_at[0], piece.text_at[1] - i as f64 * TEXT_MM * 1.6], l);
        }
        o.polyline(record, "1", &piece.boundary, true);
        o.polyline(record, "84", &piece.boundary, true);
        for p in &piece.boundary {
            extents = [extents[0].min(p[0]), extents[1].min(p[1]), extents[2].max(p[0]), extents[3].max(p[1])];
        }
        let (mut turns, mut curves) = turn_and_curve_points(&piece.boundary);
        if piece.sew_line.len() >= 3 {
            o.polyline(record, "14", &piece.sew_line, true);
            o.polyline(record, "87", &piece.sew_line, true);
            let (t, c) = turn_and_curve_points(&piece.sew_line);
            turns.extend(t);
            curves.extend(c);
        }
        for p in turns {
            o.point(record, "2", p);
        }
        for p in curves {
            o.point(record, "3", p);
        }
        for n in &piece.notches {
            // ASTM: the depth is the point's z, the width its thickness, the direction its angle;
            // a T notch's point is at its depth.
            o.entity("POINT", record, n.shape.layer(), "AcDbPoint");
            let at = if n.shape == NotchShape::T {
                let (s, c) = n.angle.to_radians().sin_cos();
                [n.at[0] + c * n.depth, n.at[1] + s * n.depth]
            } else {
                n.at
            };
            o.xyz(10, at[0], at[1], n.depth);
            if n.width > 0.0 {
                o.f(39, n.width);
            }
            o.f(50, n.angle);
        }
        if let Some([a, b]) = piece.mirror {
            o.entity("LINE", record, "6", "AcDbLine");
            o.xyz(10, a[0], a[1], 0.0);
            o.xyz(11, b[0], b[1], 0.0);
        }
        if let Some([a, b]) = piece.grain {
            o.entity("LINE", record, "7", "AcDbLine");
            o.xyz(10, a[0], a[1], 0.0);
            o.xyz(11, b[0], b[1], 0.0);
        }
        for l in piece.internal.iter().filter(|l| l.len() >= 2) {
            o.polyline(record, "8", l, false);
        }
        for d in &piece.drills {
            o.entity("POINT", record, "13", "AcDbPoint");
            o.xyz(10, d.at[0], d.at[1], d.diameter);
        }
        let entities = o.tags.split_off(start);
        blocks.push((name, record, entities));
    }
    // The style system text at the origin, then an insert of each piece.
    let mut sst = vec![
        format!("Style Name: {}", style.name),
        format!("Creation Date: {}", style.date),
        format!("Creation Time: {}", style.time),
        format!("Author: {}", style.author),
    ];
    if !style.sample_size.is_empty() {
        sst.push(format!("Sample Size: {}", style.sample_size));
    }
    sst.push("Units: METRIC".into());
    sst.push("ASTM/D13 Proposal 1 Version: D 6673-10".into());
    sst.push(format!("Curve Tolerance: {}", pf_dxf::format_real(style.tolerance)));
    let count = sst.len();
    for (i, l) in sst.iter().enumerate() {
        o.text(fixed::MODEL_RECORD, [0.0, (count - 1 - i) as f64 * TEXT_MM * 1.6], l);
    }
    for (name, _, _) in &blocks {
        o.entity("INSERT", fixed::MODEL_RECORD, "1", "AcDbBlockReference");
        o.s(2, name.clone());
        o.xyz(10, 0.0, 0.0, 0.0);
    }
    let entities = std::mem::take(&mut o.tags);
    // After the entities' handles: each piece's BLOCK and ENDBLK, then the table entries.
    let block_bases = o.next;
    let ltype = block_bases + 2 * blocks.len() as u32;
    let layer = ltype + 3;
    let text_style = layer + 1 + LAYERS.len() as u32;
    let handseed = text_style + 1;
    extents[3] = extents[3].max(count as f64 * TEXT_MM * 1.6);

    // HEADER
    o.s(0, "SECTION");
    o.s(2, "HEADER");
    let var = |o: &mut Out, name: &str| o.s(9, name);
    var(&mut o, "$ACADVER");
    o.s(1, v.acadver());
    var(&mut o, "$DWGCODEPAGE");
    o.s(3, "ANSI_1252");
    var(&mut o, "$INSBASE");
    o.xyz(10, 0.0, 0.0, 0.0);
    var(&mut o, "$EXTMIN");
    o.xyz(10, extents[0], extents[1], 0.0);
    var(&mut o, "$EXTMAX");
    o.xyz(10, extents[2], extents[3], 0.0);
    var(&mut o, "$LIMMIN");
    o.f(10, extents[0]);
    o.f(20, extents[1]);
    var(&mut o, "$LIMMAX");
    o.f(10, extents[2]);
    o.f(20, extents[3]);
    if o.handles {
        var(&mut o, "$HANDSEED");
        o.hex(5, handseed);
    }
    o.s(0, "ENDSEC");
    if o.handles {
        o.s(0, "SECTION");
        o.s(2, "CLASSES");
        o.s(0, "ENDSEC");
    }
    // TABLES
    o.s(0, "SECTION");
    o.s(2, "TABLES");
    o.table("VPORT", fixed::VPORT_TABLE, 1);
    o.record("VPORT", fixed::ACTIVE_VPORT, fixed::VPORT_TABLE, "AcDbViewportTableRecord");
    o.s(2, if o.handles { "*Active" } else { "*ACTIVE" });
    o.i(70, 0);
    let (w, h) = ((extents[2] - extents[0]).max(1.0), (extents[3] - extents[1]).max(1.0));
    o.f(10, 0.0);
    o.f(20, 0.0);
    o.f(11, 1.0);
    o.f(21, 1.0);
    o.f(12, (extents[0] + extents[2]) / 2.0);
    o.f(22, (extents[1] + extents[3]) / 2.0);
    o.f(40, h.max(w / 1.5) * 1.1);
    o.f(41, 1.5);
    o.s(0, "ENDTAB");
    o.table("LTYPE", fixed::LTYPE_TABLE, 3);
    for (i, (name, desc)) in [("ByBlock", ""), ("ByLayer", ""), ("Continuous", "Solid line")].iter().enumerate() {
        o.record("LTYPE", ltype + i as u32, fixed::LTYPE_TABLE, "AcDbLinetypeTableRecord");
        o.s(2, *name);
        o.i(70, 0);
        o.s(3, *desc);
        o.i(72, 65);
        o.i(73, 0);
        o.f(40, 0.0);
    }
    o.s(0, "ENDTAB");
    o.table("LAYER", fixed::LAYER_TABLE, 1 + LAYERS.len());
    for (i, name) in std::iter::once("0").chain(LAYERS).enumerate() {
        o.record("LAYER", layer + i as u32, fixed::LAYER_TABLE, "AcDbLayerTableRecord");
        o.s(2, name);
        o.i(70, 0);
        o.i(62, 7);
        o.s(6, "Continuous");
    }
    o.s(0, "ENDTAB");
    o.table("STYLE", fixed::STYLE_TABLE, 1);
    o.record("STYLE", text_style, fixed::STYLE_TABLE, "AcDbTextStyleTableRecord");
    o.s(2, "Standard");
    o.i(70, 0);
    o.f(40, 0.0);
    o.f(41, 1.0);
    o.f(50, 0.0);
    o.i(71, 0);
    o.f(42, 2.5);
    o.s(3, "txt");
    o.s(4, "");
    o.s(0, "ENDTAB");
    if o.handles {
        for (name, h) in [("VIEW", fixed::VIEW_TABLE), ("UCS", fixed::UCS_TABLE)] {
            o.table(name, h, 0);
            o.s(0, "ENDTAB");
        }
        o.table("APPID", fixed::APPID_TABLE, 1);
        o.record("APPID", fixed::ACAD_APPID, fixed::APPID_TABLE, "AcDbRegAppTableRecord");
        o.s(2, "ACAD");
        o.i(70, 0);
        o.s(0, "ENDTAB");
        o.table("DIMSTYLE", fixed::DIMSTYLE_TABLE, 1);
        o.record("DIMSTYLE", fixed::STANDARD_DIMSTYLE, fixed::DIMSTYLE_TABLE, "AcDbDimStyleTableRecord");
        o.s(2, "Standard");
        o.i(70, 0);
        o.s(0, "ENDTAB");
        o.table("BLOCK_RECORD", fixed::BLOCK_RECORD_TABLE, 2 + blocks.len());
        for (name, h) in [("*Model_Space", fixed::MODEL_RECORD), ("*Paper_Space", fixed::PAPER_RECORD)] {
            o.record("BLOCK_RECORD", h, fixed::BLOCK_RECORD_TABLE, "AcDbBlockTableRecord");
            o.s(2, name);
        }
        for (name, record, _) in &blocks {
            o.record("BLOCK_RECORD", *record, fixed::BLOCK_RECORD_TABLE, "AcDbBlockTableRecord");
            o.s(2, name.clone());
        }
        o.s(0, "ENDTAB");
    }
    o.s(0, "ENDSEC");
    // BLOCKS: model and paper space (from R13), then a block per piece.
    o.s(0, "SECTION");
    o.s(2, "BLOCKS");
    let block = |o: &mut Out, name: &str, record: u32, begin: u32, end: u32, body: Vec<Tag>| {
        o.object("BLOCK", begin, record, "AcDbEntity");
        o.s(8, "0");
        if o.handles {
            o.s(100, "AcDbBlockBegin");
        }
        o.s(2, name);
        o.i(70, 0);
        o.xyz(10, 0.0, 0.0, 0.0);
        o.s(3, name);
        if o.handles {
            o.s(1, "");
        }
        o.tags.extend(body);
        o.object("ENDBLK", end, record, "AcDbEntity");
        o.s(8, "0");
        if o.handles {
            o.s(100, "AcDbBlockEnd");
        }
    };
    if o.handles {
        block(&mut o, "*Model_Space", fixed::MODEL_RECORD, fixed::MODEL_BLOCK, fixed::MODEL_END, Vec::new());
        block(&mut o, "*Paper_Space", fixed::PAPER_RECORD, fixed::PAPER_BLOCK, fixed::PAPER_END, Vec::new());
    }
    for (i, (name, record, body)) in blocks.into_iter().enumerate() {
        let begin = block_bases + 2 * i as u32;
        block(&mut o, &name, record, begin, begin + 1, body);
    }
    o.s(0, "ENDSEC");
    o.s(0, "SECTION");
    o.s(2, "ENTITIES");
    o.tags.extend(entities);
    o.s(0, "ENDSEC");
    if o.handles {
        o.s(0, "SECTION");
        o.s(2, "OBJECTS");
        o.object("DICTIONARY", fixed::ROOT, 0, "AcDbDictionary");
        o.i(281, 1);
        o.s(3, "ACAD_GROUP");
        o.hex(350, fixed::GROUPS);
        o.object("DICTIONARY", fixed::GROUPS, fixed::ROOT, "AcDbDictionary");
        o.i(281, 1);
        o.s(0, "ENDSEC");
    }
    o.s(0, "EOF");
    Ok(pf_dxf::write_ascii(&o.tags))
}

/// The value of `code` in an entity's tags (the first).
fn get(e: &[Tag], code: i32) -> Option<&Value> {
    e.iter().find(|t| t.code == code).map(|t| &t.value)
}

fn num(e: &[Tag], code: i32) -> f64 {
    get(e, code).map_or(0.0, Value::as_f64)
}

fn text_of(e: &[Tag]) -> String {
    get(e, 1).map(Value::as_str).unwrap_or_default()
}

/// A DXF-AAMA file read back: its style system text and pieces (each block with a cutting line
/// on layer 1). Lightweight polylines and inserts placed away from the origin are read too.
pub fn read(bytes: &[u8]) -> Result<Style, AamaError> {
    let tags = pf_dxf::parse(bytes).map_err(|e| AamaError::Dxf(e.to_string()))?;
    let sections = pf_dxf::sections(&tags);
    let section = |name: &str| sections.iter().find(|s| s.name == name).map(|s| s.tags.as_slice()).unwrap_or(&[]);
    let mut style = Style::default();
    if let Some(i) = section("HEADER").iter().position(|t| t.code == 9 && t.str() == "$ACADVER") {
        style.version = if section("HEADER").get(i + 1).is_some_and(|t| t.str().trim() == "AC1009") { Version::R12 } else { Version::R13 };
    }
    let entities = pf_dxf::records(section("ENTITIES"));
    for (kind, e) in &entities {
        if kind != "TEXT" {
            continue;
        }
        let t = text_of(e);
        let value = |key: &str| t.strip_prefix(key).map(|v| v.trim().to_string());
        if let Some(v) = value("Style Name:") {
            style.name = v;
        } else if let Some(v) = value("Creation Date:") {
            style.date = v;
        } else if let Some(v) = value("Creation Time:") {
            style.time = v;
        } else if let Some(v) = value("Author:") {
            style.author = v;
        } else if let Some(v) = value("Sample Size:") {
            style.sample_size = v;
        } else if let Some(v) = value("Curve Tolerance:") {
            style.tolerance = v.parse().unwrap_or(0.0);
        }
    }
    // Where each block is inserted.
    let inserts: Vec<(String, Mm)> = entities
        .iter()
        .filter(|(k, _)| k == "INSERT")
        .map(|(_, e)| (get(e, 2).map(Value::as_str).unwrap_or_default(), [num(e, 10), num(e, 20)]))
        .collect();
    let records = pf_dxf::records(section("BLOCKS"));
    let mut i = 0;
    while i < records.len() {
        let (kind, head) = &records[i];
        i += 1;
        if kind != "BLOCK" {
            continue;
        }
        let name = get(head, 2).map(Value::as_str).unwrap_or_default();
        let body_start = i;
        while i < records.len() && records[i].0 != "ENDBLK" {
            i += 1;
        }
        if name.starts_with('*') {
            continue;
        }
        let at = inserts.iter().find(|(n, _)| n.eq_ignore_ascii_case(&name)).map_or([0.0, 0.0], |(_, p)| *p);
        let shift = |p: Mm| [p[0] + at[0], p[1] + at[1]];
        let mut piece = Piece { name: name.clone(), quantity: (1, 0), ..Piece::default() };
        let mut text_at = None;
        let body = records.get(body_start..i).unwrap_or(&[]);
        let mut j = 0;
        while j < body.len() {
            let (kind, e) = &body[j];
            j += 1;
            let layer = get(e, 8).map(Value::as_str).unwrap_or_default();
            match kind.as_str() {
                "TEXT" if layer == "1" => {
                    let t = text_of(e);
                    text_at.get_or_insert(shift([num(e, 10), num(e, 20)]));
                    if let Some(v) = t.strip_prefix("Piece Name:") {
                        piece.name = v.trim().to_string();
                    } else if let Some(v) = t.strip_prefix("Quantity:") {
                        let mut it = v.split(',').map(|n| n.trim().parse::<u32>().unwrap_or(0));
                        piece.quantity = (it.next().unwrap_or(1), it.next().unwrap_or(0));
                    } else if let Some(v) = t.strip_prefix("Material:") {
                        piece.material = v.trim().to_string();
                    }
                }
                "POLYLINE" | "LWPOLYLINE" => {
                    let closed = get(e, 70).is_some_and(|v| v.as_i64() & 1 == 1);
                    let mut pts = Vec::new();
                    if kind == "LWPOLYLINE" {
                        let xs = e.iter().filter(|t| t.code == 10).map(Tag::f64);
                        let ys = e.iter().filter(|t| t.code == 20).map(Tag::f64);
                        pts.extend(xs.zip(ys).map(|(x, y)| shift([x, y])));
                    } else {
                        while j < body.len() && body[j].0 == "VERTEX" {
                            pts.push(shift([num(&body[j].1, 10), num(&body[j].1, 20)]));
                            j += 1;
                        }
                    }
                    match layer.as_str() {
                        "1" if piece.boundary.is_empty() => piece.boundary = pts,
                        "14" if piece.sew_line.is_empty() => piece.sew_line = pts,
                        "8" => {
                            if closed && let Some(first) = pts.first().copied() {
                                pts.push(first);
                            }
                            piece.internal.push(pts);
                        }
                        _ => {}
                    }
                }
                "POINT" if matches!(layer.as_str(), "4" | "80" | "81" | "83") => {
                    let shape = match layer.as_str() {
                        "80" => NotchShape::T,
                        "81" => NotchShape::Castle,
                        "83" => NotchShape::U,
                        _ => NotchShape::Plain,
                    };
                    let (angle, depth) = (num(e, 50), num(e, 30));
                    let mut at = shift([num(e, 10), num(e, 20)]);
                    if shape == NotchShape::T {
                        let (s, c) = angle.to_radians().sin_cos();
                        at = [at[0] - c * depth, at[1] - s * depth];
                    }
                    piece.notches.push(Notch { at, angle, depth, width: num(e, 39), shape })
                }
                "POINT" if layer == "13" => piece.drills.push(Drill { at: shift([num(e, 10), num(e, 20)]), diameter: num(e, 30) }),
                "LINE" if layer == "6" => piece.mirror = Some([shift([num(e, 10), num(e, 20)]), shift([num(e, 11), num(e, 21)])]),
                "LINE" if layer == "7" => piece.grain = Some([shift([num(e, 10), num(e, 20)]), shift([num(e, 11), num(e, 21)])]),
                _ => {}
            }
        }
        if piece.boundary.len() >= 3 {
            piece.text_at = text_at.unwrap_or_default();
            style.pieces.push(piece);
        }
    }
    if style.pieces.is_empty() {
        return Err(AamaError::NoPieces);
    }
    Ok(style)
}

#[cfg(test)]
mod tests;
