use std::collections::BTreeMap;

use chrono::{Days, Months, NaiveDate};
use serde_json::{Value, json};

use crate::market::{CADENCES, Dividend, Price};

fn smoothed_total(regular: &[&Dividend], date: NaiveDate) -> f64 {
    (0..90)
        .map(|offset| {
            let end = date - Days::new(offset);
            let start = end - Months::new(12);
            let first = regular.partition_point(|d| d.ex_date <= start);
            let last = regular.partition_point(|d| d.ex_date <= end);
            regular[first..last].iter().map(|d| d.amount).sum::<f64>()
        })
        .sum::<f64>()
        / 90.0
}

fn latest_change(regular: &[&Dividend], date: NaiveDate) -> Option<f64> {
    let three = regular.last_chunk::<3>()?;
    let on_schedule = CADENCES.into_iter().any(|(range, _)| {
        (date - three[2].ex_date).num_days() <= *range.end()
            && three
                .windows(2)
                .all(|w| range.contains(&(w[1].ex_date - w[0].ex_date).num_days()))
    });
    (on_schedule && three[1].amount > 0.0).then(|| three[2].amount / three[1].amount - 1.0)
}

pub fn update(
    meta: &mut Value,
    prices: &BTreeMap<NaiveDate, Price>,
    dividends: &[Dividend],
    today: NaiveDate,
) {
    if let Some(summary) = meta["summary"].as_object_mut() {
        summary.remove("dividend_stats");
    }
    let Some((&date, price)) = prices.range(..=today).next_back() else {
        return;
    };
    if (today - date).num_days() > 7 {
        return;
    }
    let terms = &meta["preferred"]["dividend_terms"];
    let fixed = terms["rate_type"] == "fixed"
        && terms["fixed_annual_coupon"]
            .as_f64()
            .is_some_and(|r| r > 0.0)
        || meta["metadata"]["perpetual_note"]["fixed_annual_coupon"]
            .as_f64()
            .is_some_and(|r| r > 0.0);
    if !fixed && price.dividend_yield_ttm.is_none() {
        return;
    }
    let first = *prices.first_key_value().unwrap().0;
    let regular: Vec<_> = dividends
        .iter()
        .filter(|d| !d.special && d.ex_date <= date)
        .collect();
    if !fixed && regular.is_empty() {
        return;
    }
    let current = (!fixed).then(|| smoothed_total(&regular, date));
    let first_covered = prices
        .iter()
        .find(|(_, p)| p.dividend_yield_ttm.is_some())
        .map(|(d, _)| *d);
    let mut stats = json!({});
    for years in [1, 3, 5] {
        let past = date - Months::new(12 * years);
        let growth = match current {
            None if first <= past => Some(0.0),
            Some(current)
                if first <= past - Days::new(89) - Months::new(12)
                    && first_covered.is_some_and(|d| d <= past - Days::new(89)) =>
            {
                let previous = smoothed_total(&regular, past);
                (previous > 0.0).then(|| (current / previous).powf(1.0 / f64::from(years)) - 1.0)
            }
            _ => None,
        };
        if let Some(growth) = growth {
            stats[format!("dividend_cagr_{years}y")] = json!(rounded(growth));
        }
    }
    if fixed {
        stats["latest_regular_dividend_change"] = json!(0.0);
    } else if meta["metadata"]["payout_policy"] == "declared_rate"
        && let Some(change) = latest_change(&regular, date)
    {
        stats["latest_regular_dividend_change"] = json!(rounded(change));
    }
    if stats.as_object().is_some_and(|s| !s.is_empty()) {
        stats["as_of"] = json!(date);
        meta["summary"]["dividend_stats"] = stats;
    }
}

fn rounded(rate: f64) -> f64 {
    (rate * 1_000_000.0).round() / 1_000_000.0
}
