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

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct PerpetualNote {
    pub principal_per_note: f64,
    pub fixed_annual_coupon: f64,
}

impl PerpetualNote {
    pub fn for_security(symbol: &str, name: &str) -> Option<Self> {
        if !name
            .to_ascii_lowercase()
            .contains("perpetual subordinated notes")
        {
            return None;
        }
        Some(Self {
            principal_per_note: 25.0,
            fixed_annual_coupon: match symbol {
                "BEPH" => 0.04625,
                "BEPI" => 0.04875,
                "BEPJ" => 0.0725,
                "BIPI" => 0.05125,
                "BNJ" => 0.045,
                _ => return None,
            },
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Metadata {
    pub symbol: String,
    pub name: String,
    pub exchange: String,
    pub cik: Option<u64>,
    pub category: Category,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub perpetual_note: Option<PerpetualNote>,
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
    pub published_on: Option<NaiveDate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ffo_per_share: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ffo_total: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub affo_per_share: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub affo_total: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ownership: Option<ReitOwnership>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReitOwnership {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shares_outstanding: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outside_op_units: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outside_op_unit_share_equivalents: Option<f64>,
}

impl ReitOwnership {
    pub fn update(&mut self, incoming: reit_metrics::ownership::Ownership) {
        if let Some(shares) = incoming.shares_outstanding
            && shares.value.is_finite()
            && shares.value >= 0.0
            && shares.value < u64::MAX as f64
            && shares.value.fract() == 0.0
        {
            self.shares_outstanding = Some(shares.value as u64);
        }
        if let Some(units) = incoming.outside_op_units {
            if self.outside_op_units != Some(units.value) {
                self.outside_op_unit_share_equivalents = None;
            }
            self.outside_op_units = Some(units.value);
        }
        if let Some(equivalents) = incoming.outside_op_unit_share_equivalents {
            self.outside_op_unit_share_equivalents = Some(equivalents.value);
        }
    }
}
