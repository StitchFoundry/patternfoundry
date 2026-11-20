//! Marks for printing a pattern over home-printer pages (PF-2).
//!
//! A pattern larger than a sheet of paper prints as a grid of tiles (the host's print tiling),
//! which the sewist trims and tapes together. Each tile gets: its name (rows as letters, columns
//! as numbers: `A1` is the top-left page), the edge of its printable area, lines where its
//! neighbours' printable areas begin (lay the next page's edge on the line), and a 10 cm and a
//! 1 in test square, to check the printer kept 100% scale.
//!
//! Everything here is in points on the paper (1/72 in, y down).

use crate::Pt;
use crate::mm_to_pt;

/// A rectangle, `x0 <= x1`, `y0 <= y1`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x0: f64,
    pub y0: f64,
    pub x1: f64,
    pub y1: f64,
}

impl Rect {
    /// The rectangle between two corners. A coordinate that isn't a number is kept (and refused
    /// by whoever checks), not dropped as `f64::min` would.
    pub fn new(x0: f64, y0: f64, x1: f64, y1: f64) -> Self {
        let (x0, x1) = if x0 <= x1 { (x0, x1) } else { (x1, x0) };
        let (y0, y1) = if y0 <= y1 { (y0, y1) } else { (y1, y0) };
        Self { x0, y0, x1, y1 }
    }
    pub fn width(&self) -> f64 {
        self.x1 - self.x0
    }
    pub fn height(&self) -> f64 {
        self.y1 - self.y0
    }
    fn is_finite(&self) -> bool {
        [self.x0, self.y0, self.x1, self.y1].iter().all(|v| v.is_finite())
    }
}

/// `A`, `B`, … `Z`, `AA`, `AB`… for row 0, 1, …
pub fn row_name(row: usize) -> String {
    let mut n = row.saturating_add(1);
    let mut out = String::new();
    while n > 0 {
        n -= 1;
        out.insert(0, char::from(b'A' + (n % 26) as u8));
        n /= 26;
    }
    out
}

/// A tile's name: its row as letters, its column as a number from 1.
pub fn tile_name(col: usize, row: usize) -> String {
    format!("{}{}", row_name(row), col.saturating_add(1))
}

/// Which tile of a grid a page is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TileAt {
    pub col: usize,
    pub row: usize,
    pub cols: usize,
    pub rows: usize,
}

impl TileAt {
    /// Tile `index` (0-based, across then down) of a grid `cols` wide with `count` tiles.
    pub fn of(index: usize, count: usize, cols: usize) -> Option<Self> {
        if cols == 0 || index >= count {
            return None;
        }
        Some(Self { col: index % cols, row: index / cols, cols, rows: count.div_ceil(cols) })
    }
}

/// The marks printed on one page, in paper points.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PageMarks {
    /// Thin lines: the printable area's border, and the overlap lines where the next pages begin.
    pub lines: Vec<Vec<Pt>>,
    /// The test squares, closed (first point not repeated): 10 cm, then 1 in.
    pub squares: Vec<Vec<Pt>>,
    /// Texts: (top-left corner, size in points, text).
    pub texts: Vec<(Pt, f64, String)>,
}

/// Type size of the marks' texts, in points.
pub const TEXT_PT: f64 = 9.0;

/// The marks for a page whose printable area is `area` (paper points). `tile` is where the page is
/// in its grid (`None`: a page printed whole); `overlap` how much neighbouring tiles repeat.
pub fn page_marks(area: Rect, tile: Option<TileAt>, overlap: f64) -> PageMarks {
    let mut m = PageMarks::default();
    if !area.is_finite() || area.width() <= 0.0 || area.height() <= 0.0 {
        return m;
    }
    let overlap = if overlap.is_finite() { overlap.max(0.0) } else { 0.0 };
    let rect = |r: Rect| vec![Pt::new(r.x0, r.y0), Pt::new(r.x1, r.y0), Pt::new(r.x1, r.y1), Pt::new(r.x0, r.y1)];
    if let Some(t) = tile {
        let mut border = rect(area);
        border.push(Pt::new(area.x0, area.y0));
        m.lines.push(border);
        if overlap > 0.0 && t.col + 1 < t.cols {
            m.lines.push(vec![Pt::new(area.x1 - overlap, area.y0), Pt::new(area.x1 - overlap, area.y1)]);
        }
        if overlap > 0.0 && t.row + 1 < t.rows {
            m.lines.push(vec![Pt::new(area.x0, area.y1 - overlap), Pt::new(area.x1, area.y1 - overlap)]);
        }
        let name = format!("{}  ({} × {})", tile_name(t.col, t.row), t.rows, t.cols);
        m.texts.push((Pt::new(area.x0 + mm_to_pt(5.0), area.y1 - mm_to_pt(5.0) - TEXT_PT * 1.4), TEXT_PT, name));
    }
    // The test squares in the top-left corner of the printable area (the 1 in square alone where
    // the 10 cm one doesn't fit).
    let inset = mm_to_pt(5.0);
    let cm10 = mm_to_pt(100.0);
    let (x, y) = (area.x0 + inset, area.y0 + inset);
    let sq = |x: f64, y: f64, s: f64| rect(Rect::new(x, y, x + s, y + s));
    let label = |x: f64, y: f64, s: &str| (Pt::new(x + mm_to_pt(2.0), y + mm_to_pt(2.0)), TEXT_PT, s.to_string());
    if area.width() >= cm10 + 72.0 + 3.0 * inset && area.height() >= cm10 + 2.0 * inset {
        m.squares.push(sq(x, y, cm10));
        m.texts.push(label(x, y, "10 cm"));
        m.squares.push(sq(x + cm10 + inset, y, 72.0));
        m.texts.push(label(x + cm10 + inset, y, "1 in"));
    } else if area.width() >= 72.0 + 2.0 * inset && area.height() >= 72.0 + 2.0 * inset {
        m.squares.push(sq(x, y, 72.0));
        m.texts.push(label(x, y, "1 in"));
    }
    m
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rows_are_letters_and_columns_numbers() {
        let names: Vec<String> = [0, 1, 25, 26, 27, 701, 702].iter().map(|&r| row_name(r)).collect();
        assert_eq!(names, ["A", "B", "Z", "AA", "AB", "ZZ", "AAA"]);
        assert_eq!(tile_name(2, 1), "B3");
        assert_eq!(TileAt::of(4, 6, 3), Some(TileAt { col: 1, row: 1, cols: 3, rows: 2 }));
        assert_eq!(TileAt::of(6, 6, 3), None);
        assert_eq!(TileAt::of(0, 1, 0), None);
    }

    #[test]
    fn every_tile_has_a_10_cm_square_a_1_in_square_and_its_name() {
        // Letter's printable area with half-inch margins.
        let area = Rect::new(36.0, 36.0, 576.0, 756.0);
        let m = page_marks(area, TileAt::of(0, 6, 3), 18.0);
        assert_eq!(m.squares.len(), 2);
        let side = |s: &[Pt]| s[1].x - s[0].x;
        assert!((side(&m.squares[0]) - 7200.0 / 25.4).abs() < 1e-9);
        assert_eq!(side(&m.squares[1]), 72.0);
        assert!(m.squares.iter().flatten().all(|p| p.x > area.x0 && p.x < area.x1 && p.y > area.y0 && p.y < area.y1));
        assert!(m.texts.iter().any(|(_, _, t)| t == "A1  (2 × 3)"));
        // A1 has neighbours to the right and below: two overlap lines besides its border.
        assert_eq!(m.lines.len(), 3);
        assert!((m.lines[1][0].x - 558.0).abs() < 1e-9);
        // The last tile has none.
        assert_eq!(page_marks(area, TileAt::of(5, 6, 3), 18.0).lines.len(), 1);
    }

    #[test]
    fn a_whole_page_gets_the_squares_and_a_small_one_the_inch_alone() {
        let m = page_marks(Rect::new(0.0, 0.0, 612.0, 792.0), None, 0.0);
        assert_eq!((m.squares.len(), m.lines.len()), (2, 0));
        let small = page_marks(Rect::new(0.0, 0.0, 200.0, 200.0), None, 0.0);
        assert_eq!(small.squares.len(), 1);
        assert!(page_marks(Rect::new(0.0, 0.0, f64::NAN, 1.0), None, 0.0).squares.is_empty());
        assert!(page_marks(Rect::new(0.0, 0.0, 50.0, 50.0), None, 0.0).squares.is_empty());
    }
    #[test]
    fn test_squares_are_true_squares_of_exact_size_on_a4() {
        // A4 with 10 mm margins, in points.
        let area = Rect::new(mm_to_pt(10.0), mm_to_pt(10.0), mm_to_pt(200.0), mm_to_pt(287.0));
        let m = page_marks(area, TileAt::of(0, 1, 1), 0.0);
        assert_eq!(m.squares.len(), 2);
        let (cm, inch) = (&m.squares[0], &m.squares[1]);
        // Closed square, not repeating its first point; both sides equal; 100 mm and 72 pt exactly.
        assert_eq!(cm.len(), 4);
        assert!((cm[1].x - cm[0].x - mm_to_pt(100.0)).abs() < 1e-9);
        assert!((cm[2].y - cm[1].y - mm_to_pt(100.0)).abs() < 1e-9);
        assert!((inch[1].x - inch[0].x - 72.0).abs() < 1e-9);
        assert!((inch[2].y - inch[1].y - 72.0).abs() < 1e-9);
        // The two squares do not overlap.
        assert!(inch[0].x > cm[1].x);
    }

    #[test]
    fn a_one_by_one_grid_has_a_border_and_no_overlap_lines() {
        let area = Rect::new(36.0, 36.0, 576.0, 756.0);
        let m = page_marks(area, TileAt::of(0, 1, 1), 18.0);
        assert_eq!(m.lines.len(), 1);
        // The border is closed: it ends where it started.
        assert_eq!(m.lines[0].first(), m.lines[0].last());
    }

    #[test]
    fn a_bad_overlap_is_treated_as_none() {
        let area = Rect::new(36.0, 36.0, 576.0, 756.0);
        for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -5.0, 0.0] {
            let m = page_marks(area, TileAt::of(0, 4, 2), bad);
            assert_eq!(m.lines.len(), 1, "overlap {bad}");
        }
    }

    #[test]
    fn a_reversed_or_degenerate_area_is_handled_without_panic() {
        // Corners given backwards describe the same page.
        let a = page_marks(Rect::new(576.0, 756.0, 36.0, 36.0), None, 0.0);
        let b = page_marks(Rect::new(36.0, 36.0, 576.0, 756.0), None, 0.0);
        assert_eq!(a, b);
        // Zero-size, infinite and NaN areas give no marks at all.
        for r in [Rect::new(10.0, 10.0, 10.0, 500.0), Rect::new(0.0, 0.0, f64::INFINITY, 500.0), Rect::new(f64::NAN, 0.0, 500.0, 500.0)] {
            assert_eq!(page_marks(r, TileAt::of(0, 2, 2), 18.0), PageMarks::default());
        }
    }

    #[test]
    fn the_squares_appear_exactly_when_they_fit() {
        let inset = mm_to_pt(5.0);
        let need_w = mm_to_pt(100.0) + 72.0 + 3.0 * inset;
        let h = mm_to_pt(100.0) + 2.0 * inset;
        // Just wide enough: both. A hair narrower: the inch alone.
        assert_eq!(page_marks(Rect::new(0.0, 0.0, need_w, h), None, 0.0).squares.len(), 2);
        assert_eq!(page_marks(Rect::new(0.0, 0.0, need_w - 0.01, h), None, 0.0).squares.len(), 1);
        // Just tall enough for the inch square, then a hair short of it.
        let min = 72.0 + 2.0 * inset;
        assert_eq!(page_marks(Rect::new(0.0, 0.0, min, min), None, 0.0).squares.len(), 1);
        assert!(page_marks(Rect::new(0.0, 0.0, min - 0.01, min), None, 0.0).squares.is_empty());
    }

    #[test]
    fn names_and_grids_survive_extreme_numbers() {
        assert!(row_name(usize::MAX).chars().all(|c| c.is_ascii_uppercase()));
        assert!(!tile_name(usize::MAX, usize::MAX).is_empty());
        assert_eq!(TileAt::of(usize::MAX - 1, usize::MAX, 1).map(|t| t.rows), Some(usize::MAX));
        assert_eq!(TileAt::of(0, 0, 3), None);
        // A single column grid: every tile is its own row, none has a right neighbour.
        let area = Rect::new(36.0, 36.0, 576.0, 756.0);
        let m = page_marks(area, TileAt::of(0, 3, 1), 18.0);
        assert_eq!(m.lines.len(), 2);
        assert!((m.lines[1][0].y - (756.0 - 18.0)).abs() < 1e-9);
    }
}
