//! Grade rules (PF-6.2): grading a pattern that isn't drafted from measurements by moving its
//! points from size to size.
//!
//! Rule grading as the cutting-room standard describes it: chosen points of a piece (cardinal
//! points) carry a grade rule, how far the point moves in x and in y for each size, and the
//! rules of a pattern make its rule table (ASTM D6673, as summarised in DH Patterns and Fit's
//! overview: https://dorthehansen.com/wp-content/uploads/2014/10/ePattern-ASTM-Standard.pdf).
//! Here a rule is either the same move at every size step from the base size (a regular grade)
//! or the move of each named size from the base (an irregular one). Points between ruled points
//! move in proportion to how far along the outline they are, so a curve between two graded
//! points keeps its shape.
//!
//! Moves are in mm, x to the right and y down (the document's directions).

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::Pt;

/// Most sizes in a size range.
pub const MAX_SIZES: usize = 60;

/// The sizes a pattern is graded into, smallest first, and the size it is drawn in. One read from
/// a file is checked like one made with [`GradeSizes::new`].
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", try_from = "UncheckedSizes")]
pub struct GradeSizes {
    pub sizes: Vec<String>,
    pub base: String,
}

/// A [`GradeSizes`] as written in a file, before [`GradeSizes::new`].
#[derive(Deserialize)]
struct UncheckedSizes {
    sizes: Vec<String>,
    base: String,
}

impl TryFrom<UncheckedSizes> for GradeSizes {
    type Error = GradeError;

    fn try_from(u: UncheckedSizes) -> Result<Self, GradeError> {
        GradeSizes::new(u.sizes, &u.base)
    }
}

/// Why a size range or a rule can't be used.
#[derive(Clone, Debug, PartialEq)]
pub enum GradeError {
    NoSizes,
    TooMany,
    /// A size named twice.
    Twice(String),
    /// A base size that isn't one of the sizes.
    NoBase(String),
    /// A size's name empty or longer than 40 characters.
    Name,
    /// A move that isn't a number of mm from -500 to 500.
    Move,
}

impl std::fmt::Display for GradeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GradeError::NoSizes => write!(f, "name the sizes, smallest first"),
            GradeError::TooMany => write!(f, "a size range takes at most {MAX_SIZES} sizes"),
            GradeError::Twice(s) => write!(f, "the size `{s}` is named twice"),
            GradeError::NoBase(s) => write!(f, "the base size `{s}` is not one of the sizes"),
            GradeError::Name => write!(f, "a size's name is 1 to 40 characters"),
            GradeError::Move => write!(f, "a move is a number of mm from -500 to 500"),
        }
    }
}

impl std::error::Error for GradeError {}

impl GradeSizes {
    pub fn new(sizes: Vec<String>, base: &str) -> Result<Self, GradeError> {
        let sizes: Vec<String> = sizes.into_iter().map(|s| s.trim().to_string()).collect();
        if sizes.is_empty() {
            return Err(GradeError::NoSizes);
        }
        if sizes.len() > MAX_SIZES {
            return Err(GradeError::TooMany);
        }
        for (i, s) in sizes.iter().enumerate() {
            if s.is_empty() || s.chars().count() > 40 {
                return Err(GradeError::Name);
            }
            if sizes[..i].iter().any(|t| t.eq_ignore_ascii_case(s)) {
                return Err(GradeError::Twice(s.clone()));
            }
        }
        let base = sizes.iter().find(|s| s.eq_ignore_ascii_case(base.trim())).cloned().ok_or_else(|| GradeError::NoBase(base.trim().to_string()))?;
        Ok(Self { sizes, base })
    }

    /// The size named `name` (any case), as the range spells it.
    pub fn find(&self, name: &str) -> Option<&str> {
        self.sizes.iter().find(|s| s.eq_ignore_ascii_case(name.trim())).map(String::as_str)
    }

    /// How many sizes `size` is from the base: negative for smaller ones.
    pub fn steps(&self, size: &str) -> Option<i64> {
        let at = |n: &str| self.sizes.iter().position(|s| s.eq_ignore_ascii_case(n.trim()));
        Some(at(size)? as i64 - at(&self.base)? as i64)
    }
}

/// How a point moves from the base size. One read from a file is checked like one given to the
/// app (see [`Rule::checked`]).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", try_from = "UncheckedRule")]
pub enum Rule {
    /// The same move at every size step: `k` sizes up moves `k` times this, down the opposite.
    Step([f64; 2]),
    /// Each named size's move from the base (a size not named doesn't move).
    Sizes(BTreeMap<String, [f64; 2]>),
}

/// A [`Rule`] as written in a file, before [`Rule::checked`].
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
enum UncheckedRule {
    Step([f64; 2]),
    Sizes(BTreeMap<String, [f64; 2]>),
}

impl TryFrom<UncheckedRule> for Rule {
    type Error = GradeError;

    fn try_from(u: UncheckedRule) -> Result<Self, GradeError> {
        match u {
            UncheckedRule::Step(m) => Rule::Step(m),
            UncheckedRule::Sizes(map) => Rule::Sizes(map),
        }
        .checked()
    }
}

/// A point's grade rule: the seam line's anchor it moves.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GradeRule {
    pub anchor: usize,
    pub rule: Rule,
}

fn check(m: [f64; 2]) -> Result<[f64; 2], GradeError> {
    if m.iter().all(|v| v.is_finite() && v.abs() <= 500.0) { Ok(m) } else { Err(GradeError::Move) }
}

impl Rule {
    /// The rule with its x moves (`x`) and y moves (`y`) the other way, as for the mirror-image
    /// piece.
    pub fn mirrored(&self, x: bool, y: bool) -> Self {
        let flip = |m: [f64; 2]| [if x { -m[0] } else { m[0] }, if y { -m[1] } else { m[1] }];
        match self {
            Rule::Step(m) => Rule::Step(flip(*m)),
            Rule::Sizes(map) => Rule::Sizes(map.iter().map(|(k, v)| (k.clone(), flip(*v))).collect()),
        }
    }

    /// A checked rule.
    pub fn checked(self) -> Result<Self, GradeError> {
        Ok(match self {
            Rule::Step(m) => Rule::Step(check(m)?),
            Rule::Sizes(map) => {
                let mut out = BTreeMap::new();
                for (k, v) in map {
                    let k = k.trim().to_string();
                    if k.is_empty() || k.chars().count() > 40 {
                        return Err(GradeError::Name);
                    }
                    out.insert(k, check(v)?);
                }
                Rule::Sizes(out)
            }
        })
    }

    /// The move of `size` from the base, in mm.
    pub fn offset(&self, sizes: &GradeSizes, size: &str) -> [f64; 2] {
        match self {
            Rule::Step(m) => {
                let k = sizes.steps(size).unwrap_or(0) as f64;
                [m[0] * k, m[1] * k]
            }
            Rule::Sizes(map) => map.iter().find(|(k, _)| k.eq_ignore_ascii_case(size.trim())).map_or([0.0, 0.0], |(_, v)| *v),
        }
    }
}

/// `[x0, y0, x1, y1]` round `pts`.
fn bounds(pts: &[Pt]) -> [f64; 4] {
    pts.iter().fold([f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY], |[x0, y0, x1, y1], p| {
        [x0.min(p.x), y0.min(p.y), x1.max(p.x), y1.max(p.y)]
    })
}

/// `p` as a share of `b`'s width and height (0 to 1; 0.5 across a flat side).
fn share(p: Pt, b: [f64; 4]) -> [f64; 2] {
    let at = |v: f64, lo: f64, hi: f64| if hi - lo > 1e-9 { (v - lo) / (hi - lo) } else { 0.5 };
    [at(p.x, b[0], b[2]), at(p.y, b[1], b[3])]
}

/// The rules of the outline `from` given to the outline `to`: each ruled point's rule goes to the
/// point of `to` nearest the same place in its bounds (a share of the width and height), so a
/// front takes a back's rules though their points differ. With `mirror` x (or y), the places are
/// mirrored across the bounds and the moves turn the other way, as for the opposite half. When
/// two ruled points land on one point, the nearer keeps it.
pub fn copy_rules(from: &[Pt], rules: &[GradeRule], to: &[Pt], mirror: [bool; 2]) -> Vec<GradeRule> {
    if to.is_empty() {
        return Vec::new();
    }
    let (bf, bt) = (bounds(from), bounds(to));
    let targets: Vec<[f64; 2]> = to.iter().map(|p| share(*p, bt)).collect();
    let mut best: Vec<Option<(f64, Rule)>> = vec![None; to.len()];
    for r in rules {
        let Some(p) = from.get(r.anchor) else { continue };
        let [mut u, mut v] = share(*p, bf);
        if mirror[0] {
            u = 1.0 - u;
        }
        if mirror[1] {
            v = 1.0 - v;
        }
        let Some((i, d)) = targets.iter().map(|t| (t[0] - u).powi(2) + (t[1] - v).powi(2)).enumerate().min_by(|a, b| a.1.total_cmp(&b.1)) else {
            continue;
        };
        if best[i].as_ref().is_none_or(|(e, _)| d < *e) {
            best[i] = Some((d, r.rule.mirrored(mirror[0], mirror[1])));
        }
    }
    best.into_iter().enumerate().filter_map(|(anchor, b)| b.map(|(_, rule)| GradeRule { anchor, rule })).collect()
}

/// The move of every point of a closed outline through `points` for `size`, in mm: ruled points
/// by their rules, the points between two ruled ones in proportion to their distance along the
/// outline (by its straight runs between points). With one ruled point the whole outline moves
/// with it; with none, nothing moves.
pub fn moves(points: &[Pt], rules: &[GradeRule], sizes: &GradeSizes, size: &str) -> Vec<[f64; 2]> {
    let n = points.len();
    let mut ruled: Vec<(usize, [f64; 2])> = rules.iter().filter(|r| r.anchor < n).map(|r| (r.anchor, r.rule.offset(sizes, size))).collect();
    ruled.sort_by_key(|(i, _)| *i);
    ruled.dedup_by_key(|(i, _)| *i);
    match ruled.as_slice() {
        [] => return vec![[0.0, 0.0]; n],
        [(_, m)] => return vec![*m; n],
        _ => {}
    }
    let mut out = vec![[0.0, 0.0]; n];
    let dist = |i: usize| -> f64 {
        let (a, b) = (points[i % n], points[(i + 1) % n]);
        ((b.x - a.x).powi(2) + (b.y - a.y).powi(2)).sqrt()
    };
    for (k, &(i, mi)) in ruled.iter().enumerate() {
        let (j, mj) = ruled[(k + 1) % ruled.len()];
        out[i] = mi;
        // The points from `i` round to `j` (past the end of the outline for the last pair).
        let span = if j > i { j - i } else { j + n - i };
        let total: f64 = (0..span).map(|s| dist(i + s)).sum();
        let mut along = 0.0;
        for s in 1..span {
            along += dist(i + s - 1);
            let t = if total > 0.0 { along / total } else { s as f64 / span as f64 };
            out[(i + s) % n] = [mi[0] + (mj[0] - mi[0]) * t, mi[1] + (mj[1] - mi[1]) * t];
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sizes() -> GradeSizes {
        GradeSizes::new(vec!["S".into(), "M".into(), "L".into(), "XL".into()], "m").unwrap()
    }

    #[test]
    fn sizes_are_checked_and_counted_from_the_base() {
        let s = sizes();
        assert_eq!(s.base, "M");
        assert_eq!([s.steps("S"), s.steps("M"), s.steps("xl"), s.steps("XXL")], [Some(-1), Some(0), Some(2), None]);
        assert_eq!(GradeSizes::new(vec![], "M"), Err(GradeError::NoSizes));
        assert_eq!(GradeSizes::new(vec!["S".into(), "s".into()], "S"), Err(GradeError::Twice("s".into())));
        assert_eq!(GradeSizes::new(vec!["S".into()], "M"), Err(GradeError::NoBase("M".into())));
    }

    #[test]
    fn regular_and_irregular_rules() {
        let s = sizes();
        let step = Rule::Step([2.0, -1.0]);
        assert_eq!([step.offset(&s, "S"), step.offset(&s, "M"), step.offset(&s, "XL")], [[-2.0, 1.0], [0.0, 0.0], [4.0, -2.0]]);
        let each = Rule::Sizes(BTreeMap::from([("L".to_string(), [3.0, 0.0]), ("XL".to_string(), [7.0, 1.0])]));
        assert_eq!([each.offset(&s, "S"), each.offset(&s, "l"), each.offset(&s, "XL")], [[0.0, 0.0], [3.0, 0.0], [7.0, 1.0]]);
        assert_eq!(Rule::Step([f64::NAN, 0.0]).checked(), Err(GradeError::Move));
        assert_eq!(Rule::Step([600.0, 0.0]).checked(), Err(GradeError::Move));
    }

    #[test]
    fn points_between_ruled_points_move_in_proportion() {
        // A 100 × 50 rectangle, with points halfway along its long sides.
        let pts = [Pt::new(0.0, 0.0), Pt::new(50.0, 0.0), Pt::new(100.0, 0.0), Pt::new(100.0, 50.0), Pt::new(50.0, 50.0), Pt::new(0.0, 50.0)];
        let s = sizes();
        // The right side moves 10 mm out a size; the left side stays.
        let rules = [
            GradeRule { anchor: 0, rule: Rule::Step([0.0, 0.0]) },
            GradeRule { anchor: 2, rule: Rule::Step([10.0, 0.0]) },
            GradeRule { anchor: 3, rule: Rule::Step([10.0, 0.0]) },
            GradeRule { anchor: 5, rule: Rule::Step([0.0, 0.0]) },
        ];
        let m = moves(&pts, &rules, &s, "L");
        assert_eq!(m, [[0.0, 0.0], [5.0, 0.0], [10.0, 0.0], [10.0, 0.0], [5.0, 0.0], [0.0, 0.0]]);
        // Two sizes down: twice the step the other way.
        assert_eq!(moves(&pts, &rules, &s, "S")[2], [-10.0, 0.0]);
        // One rule moves the whole piece; none, nothing; rules past the outline are ignored.
        let one = [GradeRule { anchor: 4, rule: Rule::Step([1.0, 2.0]) }];
        assert!(moves(&pts, &one, &s, "L").iter().all(|v| *v == [1.0, 2.0]));
        assert!(moves(&pts, &[], &s, "L").iter().all(|v| *v == [0.0, 0.0]));
        let past = [GradeRule { anchor: 9, rule: Rule::Step([1.0, 2.0]) }];
        assert!(moves(&pts, &past, &s, "L").iter().all(|v| *v == [0.0, 0.0]));
    }

    #[test]
    fn rules_are_copied_to_the_nearest_points_and_mirrored() {
        // A back: a 100 × 200 rectangle, its side seam (right) graded 5 mm out a size.
        let back = [Pt::new(0.0, 0.0), Pt::new(100.0, 0.0), Pt::new(100.0, 200.0), Pt::new(0.0, 200.0)];
        let rules = [
            GradeRule { anchor: 1, rule: Rule::Step([5.0, 0.0]) },
            GradeRule { anchor: 2, rule: Rule::Sizes(BTreeMap::from([("L".to_string(), [5.0, 8.0])])) },
        ];
        // A front, drawn the other way round with an extra point at the waist: its side seam is
        // on the left.
        let front = [Pt::new(300.0, 0.0), Pt::new(250.0, 0.0), Pt::new(200.0, 0.0), Pt::new(200.0, 210.0), Pt::new(300.0, 210.0)];
        let copied = copy_rules(&back, &rules, &front, [true, false]);
        assert_eq!(
            copied,
            [
                GradeRule { anchor: 2, rule: Rule::Step([-5.0, 0.0]) },
                GradeRule { anchor: 3, rule: Rule::Sizes(BTreeMap::from([("L".to_string(), [-5.0, 8.0])])) },
            ]
        );
        // Not mirrored: the same side, the same moves.
        let same = copy_rules(&back, &rules, &back, [false, false]);
        assert_eq!(same, rules);
        assert!(copy_rules(&back, &rules, &[], [false, false]).is_empty());
    }

    /// A size range and rules read from a file are checked like ones made in the app (#202).
    #[test]
    fn a_range_and_rules_read_from_a_file_are_checked() {
        let range = |j: &str| serde_json::from_str::<GradeSizes>(j);
        assert!(range(r#"{"sizes":[],"base":"M"}"#).is_err());
        assert!(range(r#"{"sizes":["S","M"],"base":"XL"}"#).is_err());
        assert!(range(r#"{"sizes":["S","s"],"base":"S"}"#).is_err());
        assert!(
            range(&format!(r#"{{"sizes":[{}],"base":"0"}}"#, (0..=MAX_SIZES).map(|i| format!("\"{i}\"")).collect::<Vec<_>>().join(","))).is_err()
        );
        assert_eq!(range(r#"{"sizes":["S","M"],"base":"M"}"#).unwrap(), GradeSizes::new(vec!["S".into(), "M".into()], "M").unwrap());
        let rule = |j: &str| serde_json::from_str::<GradeRule>(j);
        assert!(rule(r#"{"anchor":0,"rule":{"step":[1e300,0]}}"#).is_err());
        assert!(rule(r#"{"anchor":0,"rule":{"sizes":{"L":[0,900]}}}"#).is_err());
        assert!(rule(r#"{"anchor":0,"rule":{"sizes":{"":[1,1]}}}"#).is_err());
        let ok = GradeRule { anchor: 2, rule: Rule::Sizes(BTreeMap::from([("L".to_string(), [5.0, 8.0])])) };
        assert_eq!(rule(&serde_json::to_string(&ok).unwrap()).unwrap(), ok);
        let step = GradeRule { anchor: 1, rule: Rule::Step([2.0, -1.0]) };
        assert_eq!(rule(&serde_json::to_string(&step).unwrap()).unwrap(), step);
    }

    /// Edge cases (#210): which bad values are refused, and which leave a piece unmoved.
    #[test]
    fn bad_rule_values_are_refused_and_the_limit_is_inclusive() {
        for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 500.001, -500.001, 1e300] {
            assert_eq!(Rule::Step([bad, 0.0]).checked(), Err(GradeError::Move), "{bad}");
            assert_eq!(Rule::Step([0.0, bad]).checked(), Err(GradeError::Move), "{bad}");
            assert_eq!(Rule::Sizes(BTreeMap::from([("L".to_string(), [0.0, bad])])).checked(), Err(GradeError::Move), "{bad}");
        }
        assert_eq!(Rule::Step([500.0, -500.0]).checked(), Ok(Rule::Step([500.0, -500.0])));
        // A size name is trimmed, and an empty or 41-character one is refused.
        let named = |k: &str| Rule::Sizes(BTreeMap::from([(k.to_string(), [1.0, 1.0])])).checked();
        assert_eq!(named(" L "), Ok(Rule::Sizes(BTreeMap::from([("L".to_string(), [1.0, 1.0])]))));
        assert_eq!(named("  "), Err(GradeError::Name));
        assert_eq!(named(&"x".repeat(41)), Err(GradeError::Name));
        assert!(named(&"x".repeat(40)).is_ok());
    }

    #[test]
    fn a_size_missing_from_the_chart_leaves_the_piece_unchanged() {
        let s = sizes();
        let step = Rule::Step([2.0, -1.0]);
        let each = Rule::Sizes(BTreeMap::from([("L".to_string(), [3.0, 0.0])]));
        assert_eq!(step.offset(&s, "XXL"), [0.0, 0.0]);
        assert_eq!(each.offset(&s, "XXL"), [0.0, 0.0]);
        assert_eq!(step.offset(&s, ""), [0.0, 0.0]);
        let pts = [Pt::new(0.0, 0.0), Pt::new(10.0, 0.0), Pt::new(10.0, 10.0), Pt::new(0.0, 10.0)];
        let rules = [GradeRule { anchor: 0, rule: step }, GradeRule { anchor: 2, rule: each }];
        assert!(moves(&pts, &rules, &s, "XXL").iter().all(|v| *v == [0.0, 0.0]));
    }

    #[test]
    fn a_range_of_one_size_grades_nothing_and_a_range_is_limited() {
        let one = GradeSizes::new(vec!["M".into()], "M").unwrap();
        assert_eq!(one.steps("M"), Some(0));
        assert_eq!(Rule::Step([9.0, 9.0]).offset(&one, "M"), [0.0, 0.0]);
        let pts = [Pt::new(0.0, 0.0), Pt::new(10.0, 0.0), Pt::new(10.0, 10.0)];
        let rules = [GradeRule { anchor: 0, rule: Rule::Step([9.0, 9.0]) }, GradeRule { anchor: 1, rule: Rule::Step([-4.0, 1.0]) }];
        assert!(moves(&pts, &rules, &one, "M").iter().all(|v| *v == [0.0, 0.0]));
        let many = |n: usize| GradeSizes::new((0..n).map(|i| i.to_string()).collect(), "0");
        assert!(many(MAX_SIZES).is_ok());
        assert_eq!(many(MAX_SIZES + 1), Err(GradeError::TooMany));
        assert_eq!(GradeSizes::new(vec!["  ".into()], "M"), Err(GradeError::Name));
        assert_eq!(GradeSizes::new(vec!["x".repeat(41)], "x"), Err(GradeError::Name));
    }

    #[test]
    fn grading_a_piece_with_no_points_or_one_or_two() {
        let s = sizes();
        let rule = GradeRule { anchor: 0, rule: Rule::Step([1.0, 2.0]) };
        // No points: nothing to move, and the rule's anchor is past the end.
        assert!(moves(&[], std::slice::from_ref(&rule), &s, "L").is_empty());
        assert!(moves(&[], &[], &s, "L").is_empty());
        // One point takes its rule; two rules on it count once.
        let one = [Pt::new(5.0, 5.0)];
        assert_eq!(moves(&one, std::slice::from_ref(&rule), &s, "L"), [[1.0, 2.0]]);
        assert_eq!(moves(&one, &[rule.clone(), GradeRule { anchor: 0, rule: Rule::Step([7.0, 7.0]) }], &s, "L"), [[1.0, 2.0]]);
        // Two points, each ruled: each takes its own move.
        let two = [Pt::new(0.0, 0.0), Pt::new(10.0, 0.0)];
        let rules = [rule, GradeRule { anchor: 1, rule: Rule::Step([3.0, 0.0]) }];
        assert_eq!(moves(&two, &rules, &s, "L"), [[1.0, 2.0], [3.0, 0.0]]);
        // Rules copied to or from an empty outline give none; rules past the outline are dropped.
        assert!(copy_rules(&[], &rules, &two, [false, false]).is_empty());
        assert!(copy_rules(&two, &[GradeRule { anchor: 9, rule: Rule::Step([1.0, 1.0]) }], &two, [false, false]).is_empty());
        assert_eq!(copy_rules(&one, &rules[..1], &two, [false, false]).len(), 1);
    }

    #[test]
    fn points_on_one_spot_move_without_a_nan() {
        // Every point in one place: the run has no length, so the points share the move evenly.
        let pts = [Pt::new(3.0, 3.0); 3];
        let rules = [GradeRule { anchor: 0, rule: Rule::Step([0.0, 0.0]) }, GradeRule { anchor: 2, rule: Rule::Step([4.0, 4.0]) }];
        let m = moves(&pts, &rules, &sizes(), "L");
        assert_eq!(m, [[0.0, 0.0], [2.0, 2.0], [4.0, 4.0]]);
        assert!(m.iter().flatten().all(|v| v.is_finite()));
    }

    #[test]
    fn the_last_ruled_point_wraps_round_to_the_first() {
        // A square with rules on two opposite corners: the points between them, both ways round.
        let pts = [Pt::new(0.0, 0.0), Pt::new(10.0, 0.0), Pt::new(10.0, 10.0), Pt::new(0.0, 10.0)];
        let s = sizes();
        let rules = [GradeRule { anchor: 0, rule: Rule::Step([0.0, 0.0]) }, GradeRule { anchor: 2, rule: Rule::Step([4.0, 4.0]) }];
        assert_eq!(moves(&pts, &rules, &s, "L"), [[0.0, 0.0], [2.0, 2.0], [4.0, 4.0], [2.0, 2.0]]);
    }
}
