//! Drafting from measurements (PF-3): the points of a path placed by formulas.
//!
//! A drafted path keeps, for some of its anchor points, a formula for where the point goes:
//! `x = half_bust / 2`, `y = back_length`, in millimetres from the draft's origin (the corner the
//! draft is measured from, in document coordinates, moved with the path). When a measurement
//! changes, every formula is worked out again and the points move, so the draft follows the body.
//! An axis without a formula keeps its point where the user put it.

use serde::{Deserialize, Serialize};

use crate::expr::{self, Env, ExprError};
use crate::{Pt, mm_to_pt};

/// Most points with formulas on one path.
pub const MAX_POINTS: usize = 2000;

/// Where one anchor point goes.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PointFormula {
    /// The anchor's index along the path (its first subpath).
    pub anchor: usize,
    /// Distance right of the origin, in mm (`None`: the point keeps its x).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub x: Option<String>,
    /// Distance down from the origin, in mm (`None`: the point keeps its y).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub y: Option<String>,
}

/// A path's formulas. One read from a file is made again through [`Draft::set`], so a formula or
/// a count the app would refuse is refused there too (issue 246).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "Unchecked")]
pub struct Draft {
    /// The point the formulas measure from, in document coordinates.
    pub origin: Pt,
    pub points: Vec<PointFormula>,
}

/// A [`Draft`] as written in a file, before its checks.
#[derive(Deserialize)]
struct Unchecked {
    origin: Pt,
    points: Vec<PointFormula>,
}

impl TryFrom<Unchecked> for Draft {
    type Error = DraftError;

    fn try_from(u: Unchecked) -> Result<Self, DraftError> {
        if u.points.len() > MAX_POINTS {
            return Err(DraftError::TooMany);
        }
        let mut d = Draft { origin: u.origin, points: Vec::with_capacity(u.points.len()) };
        for p in &u.points {
            d.set(p.anchor, p.x.as_deref(), p.y.as_deref())?;
        }
        Ok(d)
    }
}

/// Why a formula can't go on a point.
#[derive(Clone, Debug, PartialEq)]
pub enum DraftError {
    Formula(ExprError),
    TooMany,
}

impl std::fmt::Display for DraftError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DraftError::Formula(e) => e.fmt(f),
            DraftError::TooMany => write!(f, "a path takes formulas on at most {MAX_POINTS} points"),
        }
    }
}

impl std::error::Error for DraftError {}

/// The result of placing one point: where its axes go, in document points (`None`: kept), and
/// why an axis couldn't be worked out.
#[derive(Clone, Debug, PartialEq)]
pub struct Placed {
    pub anchor: usize,
    pub x: Option<f64>,
    pub y: Option<f64>,
    pub errors: Vec<String>,
}

impl Draft {
    /// Put formulas on `anchor` (replacing its old ones; an empty string clears an axis). A point
    /// left with no formula is dropped.
    pub fn set(&mut self, anchor: usize, x: Option<&str>, y: Option<&str>) -> Result<(), DraftError> {
        let clean = |s: Option<&str>| -> Result<Option<Option<String>>, DraftError> {
            match s {
                None => Ok(None),
                Some(t) if t.trim().is_empty() => Ok(Some(None)),
                Some(t) => {
                    expr::parse(t).map_err(DraftError::Formula)?;
                    Ok(Some(Some(t.trim().to_string())))
                }
            }
        };
        let (x, y) = (clean(x)?, clean(y)?);
        let at = self.points.iter().position(|p| p.anchor == anchor);
        if at.is_none() && self.points.len() >= MAX_POINTS {
            return Err(DraftError::TooMany);
        }
        let mut p = at.and_then(|i| self.points.get(i).cloned()).unwrap_or(PointFormula { anchor, x: None, y: None });
        if let Some(x) = x {
            p.x = x;
        }
        if let Some(y) = y {
            p.y = y;
        }
        match (at, p.x.is_some() || p.y.is_some()) {
            (Some(i), true) => {
                if let Some(slot) = self.points.get_mut(i) {
                    *slot = p;
                }
            }
            (Some(i), false) => {
                self.points.remove(i);
            }
            (None, true) => {
                self.points.push(p);
                self.points.sort_by_key(|p| p.anchor);
            }
            (None, false) => {}
        }
        Ok(())
    }

    /// Where each point goes with the measurements in `env`.
    pub fn place(&self, env: &mut Env) -> Vec<Placed> {
        self.points
            .iter()
            .map(|p| {
                let mut errors = Vec::new();
                let mut axis = |f: &Option<String>, from: f64| {
                    let text = f.as_deref()?;
                    match env.eval(text) {
                        Ok(mm) => Some(from + mm_to_pt(mm)),
                        Err(e) => {
                            errors.push(format!("{text}: {e}"));
                            None
                        }
                    }
                };
                let x = axis(&p.x, self.origin.x);
                let y = axis(&p.y, self.origin.y);
                Placed { anchor: p.anchor, x, y, errors }
            })
            .collect()
    }

    /// Forget formulas on anchors a path of `anchors` points doesn't have.
    pub fn fit(&mut self, anchors: usize) {
        self.points.retain(|p| p.anchor < anchors);
    }

    /// Move the origin by `f`, as the host transforms the path.
    pub fn transform(&mut self, f: impl Fn(Pt) -> Pt) {
        self.origin = f(self.origin);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::measure::Measurements;

    #[test]
    fn points_follow_the_measurements() {
        let mut m = Measurements::default();
        m.set_value("bust", Some(920.0)).unwrap();
        m.set_value("back_length", Some(400.0)).unwrap();
        m.set_formula("ease", Some("40")).unwrap();
        let mut d = Draft { origin: Pt::new(100.0, 50.0), points: vec![] };
        d.set(2, Some("(bust + ease) / 4"), Some("back_length")).unwrap();
        d.set(3, None, Some("back_length")).unwrap();
        let placed = d.place(&mut m.env());
        assert_eq!(placed.len(), 2);
        assert!((placed[0].x.unwrap() - (100.0 + mm_to_pt(240.0))).abs() < 1e-9);
        assert!((placed[0].y.unwrap() - (50.0 + mm_to_pt(400.0))).abs() < 1e-9);
        assert_eq!(placed[1].x, None, "no formula: the point keeps its x");
        // The bust changes, the point moves.
        m.set_value("bust", Some(1000.0)).unwrap();
        let placed = d.place(&mut m.env());
        assert!((placed[0].x.unwrap() - (100.0 + mm_to_pt(260.0))).abs() < 1e-9);
    }

    #[test]
    fn a_draft_read_from_a_file_is_checked_like_one_made_in_the_app() {
        let read = |v: serde_json::Value| serde_json::from_value::<Draft>(v);
        let good = read(serde_json::json!({"origin": {"x": 1, "y": 2}, "points": [{"anchor": 3, "x": "bust / 4"}, {"anchor": 1, "y": "10"}]}))
            .unwrap();
        assert_eq!(good.points.iter().map(|p| p.anchor).collect::<Vec<_>>(), [1, 3], "kept in order");
        let many: Vec<_> = (0..=MAX_POINTS).map(|a| serde_json::json!({"anchor": a, "x": "1"})).collect();
        for bad in [
            serde_json::json!({"origin": {"x": 0, "y": 0}, "points": many}),
            serde_json::json!({"origin": {"x": 0, "y": 0}, "points": [{"anchor": 0, "x": "1 +"}]}),
            serde_json::json!({"origin": {"x": 0, "y": 0}, "points": [{"anchor": 0, "y": "x".repeat(expr::MAX_LEN + 1)}]}),
        ] {
            assert!(read(bad.clone()).is_err(), "{}", &bad.to_string()[..80.min(bad.to_string().len())]);
        }
    }

    #[test]
    fn formulas_are_checked_cleared_and_reported() {
        let mut d = Draft::default();
        assert!(d.set(0, Some("1 +"), None).is_err());
        d.set(0, Some("hip / 4"), Some("10")).unwrap();
        let placed = d.place(&mut Measurements::default().env());
        assert_eq!(placed[0].x, None);
        assert_eq!(placed[0].y, Some(mm_to_pt(10.0)));
        assert!(placed[0].errors[0].contains("hip"), "{:?}", placed[0].errors);
        // Clearing both axes drops the point.
        d.set(0, Some(""), Some("")).unwrap();
        assert!(d.points.is_empty());
        d.set(5, Some("1"), None).unwrap();
        d.fit(3);
        assert!(d.points.is_empty());
        let back: Draft = serde_json::from_str(
            &serde_json::to_string(&Draft { origin: Pt::new(1.0, 2.0), points: vec![PointFormula { anchor: 1, x: Some("a".into()), y: None }] })
                .unwrap(),
        )
        .unwrap();
        assert_eq!(back.points[0].x.as_deref(), Some("a"));
    }
}
