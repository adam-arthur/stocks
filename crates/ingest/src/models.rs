use std::collections::{BTreeMap, BTreeSet};

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Category {
    Stock,
    Etf,
    Excluded,
    EquityReit,
    MortgageReit,
    Bdc,
    Cef,
    Preferred,
    AssetManager,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Management {
    Internal,
    External,
    Unknown,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Metadata {
    pub symbol: String,
    pub name: String,
    pub exchange: String,
    pub cik: Option<u64>,
    pub category: Category,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub management: Option<Management>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Failure {
    pub symbol: String,
    pub error: String,
    pub attempted_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FinancialsSync {
    pub updated_on: NaiveDate,
    pub companies: BTreeSet<u64>,
    pub unavailable: BTreeSet<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorporateAction {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub process_date: NaiveDate,
    #[serde(flatten, serialize_with = "serialize_action_fields")]
    pub fields: BTreeMap<String, serde_json::Value>,
}

fn serialize_action_fields<S: serde::Serializer>(
    fields: &BTreeMap<String, serde_json::Value>,
    serializer: S,
) -> std::result::Result<S::Ok, S::Error> {
    fn retain_value(value: &mut serde_json::Value) -> bool {
        match value {
            serde_json::Value::Null => false,
            serde_json::Value::String(value) => !value.is_empty(),
            serde_json::Value::Array(values) => {
                values.iter_mut().for_each(|value| {
                    retain_value(value);
                });
                !values.is_empty()
            }
            serde_json::Value::Object(fields) => {
                fields.retain(|_, value| retain_value(value));
                !fields.is_empty()
            }
            _ => true,
        }
    }

    let mut fields = fields.clone();
    fields.retain(|_, value| retain_value(value));
    fields.serialize(serializer)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReitQuarter {
    pub start: NaiveDate,
    pub end: NaiveDate,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ffo_per_share: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ffo_total: Option<i64>,
}
