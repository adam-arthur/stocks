use std::collections::BTreeMap;

use chrono::{Months, NaiveDate};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::market::Market;

#[derive(Default, Deserialize)]
pub struct Treasury {
    #[serde(rename = "DGS30")]
    pub thirty_year: BTreeMap<NaiveDate, f64>,
}

pub fn derive(
    preferred: &Value,
    note: &Value,
    market: &Market,
    treasury: &Treasury,
) -> Option<(Value, Value)> {
    let terms = &preferred["structured_dividend_terms"];
    if !market.splits.is_empty() {
        return None;
    }
    let annual_income = if note.is_object() {
        note["principal_per_note"].as_f64()? * note["fixed_annual_coupon"].as_f64()?
    } else if preferred["perpetual"] == true && terms["rate_type"] == "fixed" {
        preferred["liquidation_preference"].as_f64()? * terms["fixed_annual_coupon"].as_f64()?
    } else {
        return None;
    };
    let mut history = BTreeMap::new();
    let mut latest = None;
    for (date, price) in &market.prices {
        if let Some(percent) = treasury.thirty_year.get(date) {
            let treasury_yield = percent / 100.0;
            let current_yield = annual_income / price.close;
            let spread_bps = (current_yield - treasury_yield) * 10_000.0;
            history.insert(*date, spread_bps);
            latest = Some((date, current_yield, treasury_yield, spread_bps));
        }
    }
    let (date, current_yield, treasury_yield, spread_bps) = latest?;
    let first_date = *history.first_key_value()?.0;
    let tenth = |value: f64| (value * 10.0).round() / 10.0;
    let percentile = |start: NaiveDate| {
        let (below_or_equal, total) = history
            .range(start..)
            .fold((0usize, 0usize), |(below, total), (_, spread)| {
                (below + usize::from(*spread <= spread_bps), total + 1)
            });
        tenth(100.0 * below_or_equal as f64 / total as f64)
    };
    let trailing = |years: u32| {
        date.checked_sub_months(Months::new(12 * years))
            .filter(|start| first_date <= *start)
            .map(percentile)
    };
    let snapshot = json!({
            "as_of": date, "benchmark": "DGS30", "current_yield": crate::output::round_yield(current_yield),
            "treasury_yield": crate::output::round_yield(treasury_yield), "spread_bps": tenth(spread_bps),
            "percentile": percentile(first_date), "percentile_1y": trailing(1),
            "percentile_3y": trailing(3), "percentile_5y": trailing(5), "observations": history.len()
    });
    for spread in history.values_mut() {
        *spread = tenth(*spread);
    }
    Some((json!(history), snapshot))
}
