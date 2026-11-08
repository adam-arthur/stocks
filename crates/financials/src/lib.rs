mod common;
mod extract;
mod symbols;

use std::collections::BTreeMap;

use chrono::NaiveDate;
use serde::Serialize;

pub use extract::{extract, extract_with_rules};

#[derive(Clone, Serialize)]
pub struct Observation {
    pub value: f64,
    pub start: Option<NaiveDate>,
    pub end: NaiveDate,
    pub filed: NaiveDate,
    pub unit: String,
    #[serde(skip)]
    pub annual: bool,
}

pub type Financials = BTreeMap<String, Vec<Observation>>;

pub struct Extraction {
    pub financials: Financials,
    pub issues: Vec<Issue>,
}

#[derive(Debug)]
pub struct Issue {
    pub metric: String,
    pub end: NaiveDate,
    pub problem: &'static str,
}

pub struct Rule {
    pub metric: &'static str,
    pub unit: &'static str,
    pub duration: bool,
    pub formulas: Vec<Vec<(&'static str, f64)>>,
    pub from: Option<NaiveDate>,
    pub through: Option<NaiveDate>,
}

impl Rule {
    pub fn tags(
        metric: &'static str,
        unit: &'static str,
        duration: bool,
        tags: &[&'static str],
    ) -> Self {
        Self {
            metric,
            unit,
            duration,
            formulas: tags.iter().map(|tag| vec![(*tag, 1.0)]).collect(),
            from: None,
            through: None,
        }
    }

    pub fn includes(&self, end: NaiveDate) -> bool {
        self.from.is_none_or(|from| end >= from)
            && self.through.is_none_or(|through| end <= through)
    }
}
