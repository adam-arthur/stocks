use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AffoSupport {
    Reported,
    Unavailable,
    Unreviewed,
}

impl AffoSupport {
    pub fn is_supported(self) -> bool {
        matches!(self, Self::Reported)
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReitHistory<T> {
    pub affo_support: AffoSupport,
    pub quarters: Vec<T>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Metric {
    Ffo,
    Affo,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Unit {
    PerShare,
    Total,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Observation {
    pub metric: Metric,
    pub published_on: Option<NaiveDate>,
    pub start: NaiveDate,
    pub end: NaiveDate,
    pub label: String,
    pub value: f64,
    pub unit: Unit,
    pub scale: u64,
    pub source: String,
    pub location: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Extraction {
    pub symbol: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    pub report_period: Option<NaiveDate>,
    pub filed_on: Option<NaiveDate>,
    pub observations: Vec<Observation>,
}
