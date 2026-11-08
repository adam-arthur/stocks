use serde::{Serialize, Serializer, ser::SerializeMap, ser::SerializeSeq};
use serde_json::Value;

pub struct Document<'a>(pub &'a Value);

const ORDER: &[&str] = &[
    "start",
    "end",
    "fiscal_year_start",
    "quarter",
    "as_of",
    "metadata",
    "summary",
    "preferred",
    "symbol",
    "name",
    "category",
    "exchange",
    "cik",
    "latest_quarter_end",
    "ex_date",
    "payable_date",
    "filed",
    "value",
    "unit",
    "close",
    "closing_price",
    "market_cap",
    "shares_outstanding",
    "amount",
    "special",
    "dividend_yield",
    "price_to_earnings",
    "price_to_sales",
    "price_to_book",
    "revenue",
    "revenue_ttm",
    "earnings",
    "earnings_ttm",
    "common_shareholder_earnings",
    "common_shareholder_earnings_ttm",
    "diluted_eps",
    "diluted_earnings_per_share",
    "ffo_total",
    "ffo_per_share",
    "total_assets",
    "cash_and_equivalents",
    "total_debt",
    "net_debt_to_assets",
    "total_debt_including_preferred_stock",
    "debt_due_within_one_year",
    "debt_due_after_one_year",
    "short_term_debt",
    "commercial_paper",
    "other_short_term_debt",
    "long_term_debt",
    "long_term_debt_due_within_one_year",
    "finance_leases_due_within_one_year",
    "preferred_stock_value",
    "redeemable_preferred_stock_value",
    "preferred_shares_outstanding",
    "preferred_stock_equity",
    "shareholder_equity",
    "common_shareholder_equity",
    "temporary_shareholder_equity",
    "updated_at",
];

pub fn round_summary(summary: &mut Value) {
    for (metric, value) in summary.as_object_mut().unwrap() {
        if let Some(number) = value.as_f64() {
            *value = match metric.as_str() {
                "dividend_yield" | "net_debt_to_assets" => {
                    serde_json::json!((number * 1_000_000.0).round() / 1_000_000.0)
                }
                "closing_price" | "diluted_eps" | "price_to_earnings" | "price_to_sales"
                | "price_to_book" => {
                    serde_json::json!((number * 100.0).round() / 100.0)
                }
                _ => serde_json::json!(number.round() as i64),
            };
        }
    }
}

fn empty(value: &Value) -> bool {
    match value {
        Value::Null => true,
        Value::String(value) => value.is_empty(),
        Value::Array(value) => value.is_empty(),
        Value::Object(value) => value.values().all(empty),
        _ => false,
    }
}

impl Serialize for Document<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self.0 {
            Value::Object(value) => {
                let mut entries = value
                    .iter()
                    .filter(|(_, value)| !empty(value))
                    .collect::<Vec<_>>();
                entries.sort_by_key(|(key, _)| {
                    (
                        ORDER
                            .iter()
                            .position(|name| name == key)
                            .unwrap_or(ORDER.len()),
                        key.as_str(),
                    )
                });
                let mut map = serializer.serialize_map(Some(entries.len()))?;
                for (key, value) in entries {
                    map.serialize_entry(key, &Document(value))?;
                }
                map.end()
            }
            Value::Array(value) => {
                let mut seq = serializer.serialize_seq(Some(value.len()))?;
                for value in value {
                    seq.serialize_element(&Document(value))?;
                }
                seq.end()
            }
            value => value.serialize(serializer),
        }
    }
}
