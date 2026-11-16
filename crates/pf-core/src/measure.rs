//! Measurements (PF-3): the body measurements a pattern is drafted from, the values derived from
//! them, and size charts.
//!
//! A measurement set holds named values in millimetres (`bust`, `waist`, `back_length`…) and named
//! formulas over them (`ease = 40`, `half_bust = (bust + ease) / 2`). Names are letters, digits
//! and `_`, starting with a letter. A size chart is a table of the same names, one row per size;
//! choosing a size copies its row into the set.
//!
//! **Personal data.** A set of someone's own measurements stays on their computer: the host keeps
//! it in the document (their own file) and never logs it or embeds it in files made to share.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::expr::{self, Env, ExprError};

/// Most measurements and formulas in a set, and sizes in a chart.
pub const MAX_ENTRIES: usize = 500;
/// The largest measurement accepted, in mm (10 m: any body measurement, with room to spare).
pub const MAX_MM: f64 = 10_000.0;

/// Why a value can't go into a measurement set.
#[derive(Clone, Debug, PartialEq)]
pub enum MeasureError {
    /// Not a letter or `_` then letters, digits or `_` (at most 64), or `pi`.
    Name(String),
    /// A measurement below 0 or above [`MAX_MM`], or not a number.
    Value(String, f64),
    /// A formula that doesn't parse.
    Formula(String, ExprError),
    /// More than [`MAX_ENTRIES`].
    TooMany,
    /// A size chart's text that isn't a table of sizes.
    Chart(String),
}

impl std::fmt::Display for MeasureError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MeasureError::Name(n) => write!(f, "`{n}` can't be a measurement's name: use letters, digits and _, starting with a letter"),
            MeasureError::Value(n, v) => write!(f, "{n}: {v} mm is not a measurement (0 to {MAX_MM} mm)"),
            MeasureError::Formula(n, e) => write!(f, "{n}: {e}"),
            MeasureError::TooMany => write!(f, "a measurement set takes at most {MAX_ENTRIES} entries"),
            MeasureError::Chart(why) => write!(f, "not a size chart: {why}"),
        }
    }
}

impl std::error::Error for MeasureError {}

fn check_name(name: &str) -> Result<(), MeasureError> {
    if expr::valid_name(name) { Ok(()) } else { Err(MeasureError::Name(name.to_string())) }
}

fn check_mm(name: &str, mm: f64) -> Result<f64, MeasureError> {
    if mm.is_finite() && (0.0..=MAX_MM).contains(&mm) { Ok(mm) } else { Err(MeasureError::Value(name.to_string(), mm)) }
}

/// A set of measurements and formulas.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Measurements {
    /// Whose or which: "Me", "Size 12"…
    #[serde(default)]
    pub name: String,
    /// Measured values, in mm.
    #[serde(default)]
    pub values: BTreeMap<String, f64>,
    /// Derived values: a formula over the measurements (in mm).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub formulas: BTreeMap<String, String>,
    /// The size chart the values came from, and the size, when they came from one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<String>,
    /// The measured values may go into the copies of the document that exported files carry (a
    /// PDF or SVG saved with Preserve Editing). Off by default: personal measurements stay on the
    /// user's computer unless they choose to share them.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub share: bool,
    /// Whose proportions the avatar fills in the rest from (PF-10.11).
    #[serde(default, skip_serializing_if = "is_female")]
    pub figure: crate::avatar::Figure,
}

fn is_female(f: &crate::avatar::Figure) -> bool {
    *f == crate::avatar::Figure::Female
}

impl Measurements {
    /// The set as files made to share carry it: without the measured values unless [`Self::share`]
    /// (the formulas, which are the pattern's, stay).
    pub fn for_sharing(&self) -> Self {
        if self.share { self.clone() } else { Self { values: BTreeMap::new(), ..self.clone() } }
    }

    fn count(&self) -> usize {
        self.values.len() + self.formulas.len()
    }

    /// Set measurement `name` to `mm`, or (`None`) remove it.
    pub fn set_value(&mut self, name: &str, mm: Option<f64>) -> Result<(), MeasureError> {
        check_name(name)?;
        match mm {
            None => {
                self.values.remove(name);
            }
            Some(v) => {
                let v = check_mm(name, v)?;
                if !self.values.contains_key(name) && self.count() >= MAX_ENTRIES {
                    return Err(MeasureError::TooMany);
                }
                self.formulas.remove(name);
                self.values.insert(name.to_string(), v);
            }
        }
        Ok(())
    }

    /// Set formula `name` to `text`, or (`None`) remove it. A formula replaces a measured value of
    /// the same name.
    pub fn set_formula(&mut self, name: &str, text: Option<&str>) -> Result<(), MeasureError> {
        check_name(name)?;
        match text {
            None => {
                self.formulas.remove(name);
            }
            Some(t) => {
                expr::parse(t).map_err(|e| MeasureError::Formula(name.to_string(), e))?;
                if !self.formulas.contains_key(name) && !self.values.contains_key(name) && self.count() >= MAX_ENTRIES {
                    return Err(MeasureError::TooMany);
                }
                self.values.remove(name);
                self.formulas.insert(name.to_string(), t.trim().to_string());
            }
        }
        Ok(())
    }

    /// The set as an expression environment: measurements and formulas, in mm.
    pub fn env(&self) -> Env {
        let mut env = Env::new();
        for (k, v) in &self.values {
            env.insert(k, &format!("{v:?}"));
        }
        for (k, f) in &self.formulas {
            env.insert(k, f);
        }
        env
    }

    /// Every measurement and formula worked out, in mm (or why it can't be).
    pub fn computed(&self) -> BTreeMap<String, Result<f64, ExprError>> {
        let mut env = self.env();
        self.values.keys().chain(self.formulas.keys()).map(|k| (k.clone(), env.value(k))).collect()
    }
}

/// A size chart: the same measurements for each size.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SizeChart {
    #[serde(default)]
    pub name: String,
    /// The measurements' names, in the chart's column order.
    pub columns: Vec<String>,
    /// Each size: its name and its measurements in mm, in [`Self::columns`]' order (`None`: not given).
    pub sizes: Vec<(String, Vec<Option<f64>>)>,
}

impl SizeChart {
    /// Read a chart from comma- (or semicolon- or tab-) separated text: a header row naming the
    /// size column and each measurement (`size, bust, waist, hip`), then a row per size. Values are
    /// in `unit` (millimetres per value: 1 for mm, 10 for cm, 25.4 for inches); an empty cell is
    /// a measurement the size doesn't give.
    pub fn from_text(name: &str, text: &str, mm_per_unit: f64) -> Result<Self, MeasureError> {
        if !(mm_per_unit.is_finite() && mm_per_unit > 0.0) {
            return Err(MeasureError::Chart("the unit isn't a length".into()));
        }
        // Each line with its number in the text (from 1), so an error names the line it is on.
        let mut lines = text.lines().map(str::trim).enumerate().filter(|(_, l)| !l.is_empty() && !l.starts_with('#'));
        let (_, header) = lines.next().ok_or_else(|| MeasureError::Chart("it is empty".into()))?;
        let sep = [',', ';', '\t'].into_iter().max_by_key(|c| header.matches(*c).count()).unwrap_or(',');
        let cells = |l: &str| l.split(sep).map(|c| c.trim().trim_matches('"').trim().to_string()).collect::<Vec<_>>();
        let head = cells(header);
        let Some((_, names)) = head.split_first() else { return Err(MeasureError::Chart("no columns".into())) };
        if names.is_empty() {
            return Err(MeasureError::Chart("the header names no measurements".into()));
        }
        let columns: Vec<String> = names.iter().map(|n| n.to_ascii_lowercase().replace([' ', '-'], "_")).collect();
        for c in &columns {
            check_name(c)?;
        }
        if columns.len() > MAX_ENTRIES {
            return Err(MeasureError::TooMany);
        }
        let mut sizes = Vec::new();
        for (i, line) in lines {
            if sizes.len() >= MAX_ENTRIES {
                return Err(MeasureError::TooMany);
            }
            let row = cells(line);
            let Some((size, vals)) = row.split_first() else { continue };
            let mut out = Vec::with_capacity(columns.len());
            for (j, c) in columns.iter().enumerate() {
                let cell = vals.get(j).map(String::as_str).unwrap_or("");
                if cell.is_empty() {
                    out.push(None);
                    continue;
                }
                let v: f64 = cell.replace(',', ".").parse().map_err(|_| MeasureError::Chart(format!("row {}: `{cell}` is not a number", i + 1)))?;
                out.push(Some(check_mm(c, v * mm_per_unit)?));
            }
            sizes.push((size.clone(), out));
        }
        if sizes.is_empty() {
            return Err(MeasureError::Chart("it has no sizes".into()));
        }
        Ok(Self { name: name.trim().to_string(), columns, sizes })
    }

    /// The sizes' names.
    pub fn size_names(&self) -> Vec<&str> {
        self.sizes.iter().map(|(s, _)| s.as_str()).collect()
    }

    /// Copy size `size`'s measurements into `m` (the ones the size gives; formulas stay).
    pub fn apply(&self, size: &str, m: &mut Measurements) -> Result<(), MeasureError> {
        let (name, vals) = self
            .sizes
            .iter()
            .find(|(s, _)| s.eq_ignore_ascii_case(size.trim()))
            .ok_or_else(|| MeasureError::Chart(format!("it has no size `{size}`")))?;
        for (c, v) in self.columns.iter().zip(vals) {
            if let Some(v) = v {
                m.set_value(c, Some(*v))?;
            }
        }
        m.size = Some(if self.name.is_empty() { name.clone() } else { format!("{} {name}", self.name) });
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn measurements_and_formulas_work_out_in_mm() {
        let mut m = Measurements::default();
        m.set_value("bust", Some(920.0)).unwrap();
        m.set_value("waist", Some(740.0)).unwrap();
        m.set_formula("ease", Some("40")).unwrap();
        m.set_formula("half_bust", Some("(bust + ease) / 2")).unwrap();
        let c = m.computed();
        assert_eq!(c["half_bust"], Ok(480.0));
        assert_eq!(c["waist"], Ok(740.0));
        // Changing the bust changes what is drawn from it.
        m.set_value("bust", Some(1000.0)).unwrap();
        assert_eq!(m.computed()["half_bust"], Ok(520.0));
        // A formula of a missing measurement says so; the others still work.
        m.set_formula("hip_line", Some("hip / 4")).unwrap();
        assert!(matches!(m.computed()["hip_line"], Err(ExprError::Unknown(_))));
        assert_eq!(m.computed()["waist"], Ok(740.0));
        // A set round-trips.
        let back: Measurements = serde_json::from_str(&serde_json::to_string(&m).unwrap()).unwrap();
        assert_eq!(back, m);
    }

    #[test]
    fn bad_names_values_and_formulas_are_refused() {
        let mut m = Measurements::default();
        assert!(m.set_value("2bust", Some(900.0)).is_err());
        assert!(m.set_value("bust size", Some(900.0)).is_err());
        assert!(m.set_value("bust", Some(-1.0)).is_err());
        assert!(m.set_value("bust", Some(f64::NAN)).is_err());
        assert!(m.set_value("bust", Some(20_000.0)).is_err());
        assert!(m.set_formula("ease", Some("4 +")).is_err());
        m.set_formula("a", Some("b")).unwrap();
        m.set_formula("b", Some("a")).unwrap();
        assert!(matches!(m.computed()["a"], Err(ExprError::Cycle(_))));
        for i in 0..MAX_ENTRIES {
            m.set_value(&format!("m{i}"), Some(1.0)).ok();
        }
        assert_eq!(m.set_value("one_more", Some(1.0)), Err(MeasureError::TooMany));
        // Removing works.
        m.set_value("m0", None).unwrap();
        assert!(!m.values.contains_key("m0"));
    }

    #[test]
    fn a_size_chart_is_read_from_a_table_and_fills_in_a_size() {
        let text = "# Example chart (made up for the test)\nSize; Bust; Waist; Hip\n10; 86; 68; 92\n12; 91; 73; 97\n14; 96; 78;\n";
        let chart = SizeChart::from_text("Test", text, 10.0).unwrap();
        assert_eq!(chart.columns, ["bust", "waist", "hip"]);
        assert_eq!(chart.size_names(), ["10", "12", "14"]);
        let mut m = Measurements::default();
        m.set_formula("ease", Some("40")).unwrap();
        chart.apply("12", &mut m).unwrap();
        assert_eq!((m.values["bust"], m.values["hip"], m.size.as_deref()), (910.0, 970.0, Some("Test 12")));
        chart.apply("14", &mut m).unwrap();
        assert_eq!((m.values["bust"], m.values["hip"]), (960.0, 970.0), "a size without a hip keeps the one there");
        assert!(m.formulas.contains_key("ease"));
        assert!(chart.apply("99", &mut m).is_err());
        // Inches, commas.
        let chart = SizeChart::from_text("", "size,bust\nS,34\n", 25.4).unwrap();
        assert!((chart.sizes[0].1[0].unwrap() - 863.6).abs() < 1e-9);
        assert!(SizeChart::from_text("", "", 1.0).is_err());
        assert!(SizeChart::from_text("", "size,bust\n", 1.0).is_err());
        assert!(SizeChart::from_text("", "size,bust\nS,big\n", 1.0).is_err());
        assert!(SizeChart::from_text("", "size,2x\nS,1\n", 1.0).is_err());
        assert!(SizeChart::from_text("", "size,bust\nS,1\n", 0.0).is_err());
    }

    #[test]
    fn a_size_chart_error_names_the_line_it_is_on() {
        // Line 5 of the text, after a comment and a blank line.
        let text = "# Example chart (made up for the test)\nsize,bust\n\nS,86\nM,big\n";
        let e = SizeChart::from_text("", text, 10.0).unwrap_err().to_string();
        assert!(e.contains("row 5:"), "{e}");
    }
}
