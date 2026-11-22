use std::collections::BTreeMap;

use anyhow::{Context, Result, ensure};
use chrono::{Months, NaiveDate};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Deserialize, Serialize)]
pub struct Dividend {
    #[serde(default, skip_serializing)]
    pub id: String,
    pub ex_date: NaiveDate,
    pub payable_date: Option<NaiveDate>,
    pub amount: f64,
    pub special: bool,
}

#[derive(Deserialize, Serialize)]
pub struct Price {
    pub close: f64,
    pub dividend_yield_ttm: Option<f64>,
    pub dividend_yield_fwd: Option<f64>,
}

#[derive(Default)]
pub struct Market {
    pub prices: BTreeMap<NaiveDate, Price>,
    pub dividends: Vec<Dividend>,
    pub splits: Vec<(NaiveDate, f64)>,
}

pub fn factor(splits: &[(NaiveDate, f64)], date: NaiveDate, through: NaiveDate) -> f64 {
    splits
        .iter()
        .filter(|(ex, _)| *ex > date && *ex <= through)
        .map(|(_, ratio)| ratio)
        .product()
}

pub(crate) const CADENCES: [(std::ops::RangeInclusive<i64>, i64); 5] = [
    (5..=9, 52),
    (24..=38, 12),
    (75..=110, 4),
    (150..=215, 2),
    (330..=400, 1),
];

fn indicated_rate(history: &[Dividend]) -> Option<(NaiveDate, i64, f64)> {
    let mut regular = history.iter().rev().filter(|dividend| !dividend.special);
    let (latest, second, first) = (regular.next()?, regular.next()?, regular.next()?);
    let older_gap = (second.ex_date - first.ex_date).num_days();
    let newer_gap = (latest.ex_date - second.ex_date).num_days();
    CADENCES
        .into_iter()
        .find(|(days, _)| days.contains(&older_gap) && days.contains(&newer_gap))
        .map(|(days, frequency)| {
            (
                latest.ex_date,
                *days.end(),
                latest.amount * frequency as f64,
            )
        })
}

pub fn derive(
    prices: BTreeMap<NaiveDate, f64>,
    actions: &[Value],
    synced: Option<NaiveDate>,
    as_of: NaiveDate,
) -> Result<Market> {
    let Some(start) =
        alpaca_client::price_history_start(prices.range(..=as_of).map(|(date, _)| *date))
    else {
        return Ok(Market::default());
    };
    let mut splits = Vec::new();
    for action in actions.iter().filter(|a| {
        matches!(
            a["type"].as_str(),
            Some("forward_splits" | "reverse_splits")
        )
    }) {
        let date = serde_json::from_value::<NaiveDate>(action["ex_date"].clone())
            .context("split missing ex_date")?;
        if date < start || date > as_of {
            continue;
        }
        let ratio = action["new_rate"]
            .as_f64()
            .context("split missing new_rate")?
            / action["old_rate"]
                .as_f64()
                .context("split missing old_rate")?;
        ensure!(ratio.is_finite() && ratio > 0.0, "invalid split ratio");
        splits.push((date, ratio));
    }
    splits.sort_by_key(|(date, _)| *date);
    let mut dividends = Vec::new();
    for action in actions.iter().filter(|a| a["type"] == "cash_dividends") {
        let ex_date = serde_json::from_value::<NaiveDate>(action["ex_date"].clone())
            .context("dividend missing ex_date")?;
        if ex_date < start || ex_date > as_of {
            continue;
        }
        let amount = action["rate"].as_f64().context("dividend missing rate")?
            / factor(&splits, ex_date, as_of);
        ensure!(amount.is_finite() && amount >= 0.0, "invalid dividend rate");
        dividends.push(Dividend {
            id: action["id"].as_str().context("dividend missing id")?.into(),
            ex_date,
            payable_date: action
                .get("payable_date")
                .map(|v| serde_json::from_value(v.clone()))
                .transpose()?,
            amount,
            special: action["special"]
                .as_bool()
                .context("dividend missing special flag")?,
        });
    }
    dividends.sort_by(|a, b| (a.ex_date, &a.id).cmp(&(b.ex_date, &b.id)));
    let mut result = BTreeMap::new();
    let (mut first, mut last, mut regular) = (0, 0, 0.0);
    let mut indicated = None;
    for (date, raw) in prices
        .into_iter()
        .filter(|(date, _)| *date >= start && *date <= as_of)
    {
        let close = raw / factor(&splits, date, as_of);
        ensure!(close.is_finite() && close > 0.0, "invalid close on {date}");
        while last < dividends.len() && dividends[last].ex_date <= date {
            if !dividends[last].special {
                regular += dividends[last].amount;
                indicated = indicated_rate(&dividends[..=last]);
            }
            last += 1;
        }
        while first < last
            && dividends[first].ex_date <= date.checked_sub_months(Months::new(12)).unwrap()
        {
            if !dividends[first].special {
                regular -= dividends[first].amount;
            }
            first += 1;
        }
        let covered = date >= NaiveDate::from_ymd_opt(2017, 1, 1).unwrap()
            && synced.is_some_and(|sync| sync >= date);
        result.insert(
            date,
            Price {
                close,
                dividend_yield_ttm: covered.then_some(regular.max(0.0) / close),
                dividend_yield_fwd: indicated
                    .filter(|(ex_date, max_gap, _)| {
                        covered && (date - *ex_date).num_days() <= *max_gap
                    })
                    .map(|(_, _, rate)| rate / close),
            },
        );
    }
    Ok(Market {
        prices: result,
        dividends,
        splits,
    })
}
