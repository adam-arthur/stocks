use crate::state::{Alert, Notification, State};
use anyhow::{Context, Result};
use chrono::{Days, Months, NaiveDate};
use serde::Deserialize;
use std::{collections::BTreeMap, fs, io::ErrorKind, path::Path};

const RULE: &str = "trailing_dividend_yield_5y_percentile";
const MIN_PERCENTILE: f64 = 95.0;

#[derive(Deserialize)]
struct Price {
    dividend_yield_ttm: Option<f64>,
}

impl Price {
    fn yield_for_ranking(&self) -> Option<f64> {
        self.dividend_yield_ttm
            .filter(|&y| y >= 0.0)
            .map(|y| (y * 1_000_000.0).round() / 1_000_000.0)
    }
}

fn ranked_yield(path: &Path, symbol: String, today: NaiveDate) -> Result<Option<Alert>> {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.into()),
    };
    let prices: BTreeMap<NaiveDate, Price> = serde_json::from_slice(&bytes)?;
    let Some((&date, price)) = prices.range(..=today).next_back() else {
        return Ok(None);
    };
    let Some(current) = price.yield_for_ranking() else {
        return Ok(None);
    };
    if (today - date).num_days() > 7 {
        return Ok(None);
    }
    let start = date - Months::new(60);
    let mut history = prices
        .range(start..date)
        .filter_map(|(day, price)| price.yield_for_ranking().map(|y| (day, y)))
        .peekable();
    if history
        .peek()
        .is_none_or(|(day, _)| **day > start + Days::new(7))
    {
        return Ok(None);
    }
    let mut values: Vec<_> = history.map(|(_, value)| value).collect();
    if values.len() < 1200 {
        return Ok(None);
    }
    values.sort_unstable_by(f64::total_cmp);
    let n = values.len();
    let below = values.partition_point(|&value| value < current);
    let at_or_below = values.partition_point(|&value| value <= current);
    let median = (values[(n - 1) / 2] + values[n / 2]) / 2.0;
    let std_devs_from_median = (values[0] != values[n - 1]).then(|| {
        let mean = values.iter().sum::<f64>() / n as f64;
        let variance = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / n as f64;
        (current - median) / variance.sqrt()
    });
    Ok(Some(Alert {
        symbol,
        rule: RULE.into(),
        observed_on: date,
        value: current,
        percentile: 50.0 * (below + at_or_below) as f64 / n as f64,
        std_devs_from_median,
    }))
}

pub fn evaluate(state: &mut State, stocks: &Path, today: NaiveDate) -> Result<()> {
    let first_run = !state.active.contains_key(RULE);
    let active = state.active.entry(RULE.into()).or_default();
    let mut alerts = Vec::new();
    for entry in fs::read_dir(stocks)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let path = entry.path().join("prices.json");
        let symbol = entry
            .file_name()
            .into_string()
            .map_err(|_| anyhow::anyhow!("invalid ticker at {}", path.display()))?;
        let Some(alert) =
            ranked_yield(&path, symbol, today).with_context(|| path.display().to_string())?
        else {
            continue;
        };
        if alert.value == 0.0 || alert.percentile < MIN_PERCENTILE {
            active.remove(&alert.symbol);
        } else if active.insert(alert.symbol.clone()) {
            alerts.push(alert);
        }
    }
    alerts.sort_by(|a, b| {
        b.percentile
            .total_cmp(&a.percentile)
            .then_with(|| a.symbol.cmp(&b.symbol))
    });
    if first_run && !alerts.is_empty() {
        state
            .pending
            .push_back(Notification::InitialSummary(alerts));
    } else {
        state
            .pending
            .extend(alerts.into_iter().map(Notification::Entered));
    }
    Ok(())
}
