use std::{collections::BTreeMap, fs, path::Path};

use anyhow::{Context, Result};
use chrono::{Days, Months, NaiveDate, Utc};
use serde_json::{Value, json};

use crate::{
    market::Price,
    preferred_spread::{self, Treasury},
};

fn stats(series: &[(NaiveDate, f64)], today: NaiveDate, bps_per_unit: f64) -> Option<Value> {
    let (&(date, current), prior) = series.split_last()?;
    if !(0..=7).contains(&(today - date).num_days()) {
        return None;
    }
    if prior.is_empty() {
        return None;
    }
    let mut result = json!({"as_of": date, "value": current});
    add_period(&mut result, prior, current, bps_per_unit, "");
    for (years, suffix) in [(1, "_1y"), (3, "_3y"), (5, "_5y")] {
        let start = date - Months::new(years * 12);
        let window = &prior[prior.partition_point(|(day, _)| *day < start)..];
        if window.len() >= years as usize * 240 && window[0].0 <= start + Days::new(7) {
            add_period(&mut result, window, current, bps_per_unit, suffix);
        }
    }
    Some(result)
}

fn add_period(
    result: &mut Value,
    prior: &[(NaiveDate, f64)],
    current: f64,
    bps_per_unit: f64,
    suffix: &str,
) {
    let mut values: Vec<_> = prior.iter().map(|(_, v)| *v).collect();
    values.sort_unstable_by(f64::total_cmp);
    let n = values.len();
    let median = (values[(n - 1) / 2] + values[n / 2]) / 2.0;
    let difference = current - median;
    let below = values.partition_point(|&v| v < current);
    let at_or_below = values.partition_point(|&v| v <= current);
    let distance = (values[0] != values[n - 1]).then(|| {
        let mean = values.iter().sum::<f64>() / n as f64;
        let variance = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / n as f64;
        difference / variance.sqrt()
    });
    result[format!("median{suffix}")] = json!(median);
    result[format!("latest_minus_median_bps{suffix}")] =
        json!((difference * bps_per_unit * 1000.0).round() / 1000.0);
    result[format!("percentile{suffix}")] = json!(50.0 * (below + at_or_below) as f64 / n as f64);
    if let Some(distance) = distance {
        result[format!("std_devs_from_median{suffix}")] = json!(distance);
    }
}

pub fn update(
    meta: &mut Value,
    prices: &BTreeMap<NaiveDate, Price>,
    treasury: &Treasury,
    today: NaiveDate,
) {
    let annual = meta["summary"]["preferred_spread"]
        .is_object()
        .then(|| {
            preferred_spread::annual_income(
                &meta["preferred"],
                &meta["preferred"]["dividend_terms"],
                &meta["metadata"]["perpetual_note"],
            )
        })
        .flatten();
    if let Some(summary) = meta["summary"].as_object_mut() {
        summary.remove("yield_stats_5y");
        summary.remove("yield_stats");
    }
    let mut yields = Vec::new();
    let mut spreads = Vec::new();
    for (&date, price) in prices.range(..=today) {
        let paired = treasury.thirty_year.get(&date);
        let value = annual
            .map(|amount| amount / price.close)
            .or(price.dividend_yield_ttm);
        let Some(value) = value.filter(|v| v.is_finite() && *v >= 0.0) else {
            continue;
        };
        let value = (value * 1_000_000.0).round() / 1_000_000.0;
        yields.push((date, value));
        if let Some(rate) = paired {
            spreads.push((
                date,
                ((value * 10_000.0 - rate * 100.0) * 100.0).round() / 100.0,
            ));
        }
    }
    if yields.last().map(|p| &p.0) != prices.range(..=today).next_back().map(|p| p.0)
        || yields.last().is_none_or(|p| p.1 == 0.0)
    {
        return;
    }
    let yield_stats = stats(&yields, today, 10_000.0);
    let spread_stats = stats(&spreads, today, 1.0);
    if yield_stats.is_some() || spread_stats.is_some() {
        meta["summary"]["yield_stats"] = json!({
            "yield_basis": if annual.is_some() { "fixed_coupon" } else { "trailing_dividends" },
            "benchmark": "DGS30"
        });
        if let Some(stats) = yield_stats {
            meta["summary"]["yield_stats"]["yield"] = stats;
        }
        if let Some(stats) = spread_stats {
            meta["summary"]["yield_stats"]["spread_bps"] = stats;
        }
    }
}

pub fn refresh(output: &Path) -> Result<usize> {
    let _lock = crate::lock_output(output)?;
    let treasury =
        crate::read(&output.join("treasury.json"))?.context("missing derived treasury.json")?;
    let today = Utc::now().date_naive();
    let mut changed = 0;
    for entry in fs::read_dir(output.join("stocks"))? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let path = entry.path().join("meta.json");
        let mut meta: Value = crate::read(&path)?.with_context(|| path.display().to_string())?;
        let had_five_year_summary = meta["summary"].get("yield_stats_5y").is_some();
        let old = meta["summary"]["yield_stats"].clone();
        let old_dividends = meta["summary"]["dividend_stats"].clone();
        let prices = entry.path().join("prices.json");
        let prices = crate::read(&prices)?.with_context(|| prices.display().to_string())?;
        let dividends = entry.path().join("dividends.json");
        let dividends =
            crate::read::<Vec<_>>(&dividends)?.with_context(|| dividends.display().to_string())?;
        crate::dividend_stats::update(&mut meta, &prices, &dividends, today);
        update(&mut meta, &prices, &treasury, today);
        if had_five_year_summary
            || meta["summary"]["yield_stats"] != old
            || meta["summary"]["dividend_stats"] != old_dividends
        {
            crate::write(&path, &meta)?;
            changed += 1;
        }
    }
    Ok(changed)
}
