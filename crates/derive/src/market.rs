use std::collections::BTreeMap;

use anyhow::{Context, Result, ensure};
use chrono::{Months, NaiveDate};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Deserialize, Serialize)]
pub struct Dividend {
    #[serde(skip_serializing)]
    pub id: String,
    pub ex_date: NaiveDate,
    pub payable_date: Option<NaiveDate>,
    pub amount: f64,
    pub special: bool,
}

#[derive(Serialize)]
pub struct Price {
    pub close: f64,
    pub dividend_yield: Option<f64>,
    pub dividend_yield_excluding_specials: Option<f64>,
}

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

pub fn derive(
    prices: BTreeMap<NaiveDate, f64>,
    actions: &[Value],
    synced: Option<NaiveDate>,
    as_of: NaiveDate,
) -> Result<Market> {
    let mut splits = Vec::new();
    for action in actions.iter().filter(|a| {
        matches!(
            a["type"].as_str(),
            Some("forward_splits" | "reverse_splits")
        )
    }) {
        let date = serde_json::from_value::<NaiveDate>(action["ex_date"].clone())
            .context("split missing ex_date")?;
        if date > as_of {
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
        if ex_date > as_of {
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
    let (mut first, mut last, mut total, mut regular) = (0, 0, 0.0, 0.0);
    for (date, raw) in prices.into_iter().filter(|(date, _)| *date <= as_of) {
        let close = raw / factor(&splits, date, as_of);
        ensure!(close.is_finite() && close > 0.0, "invalid close on {date}");
        while last < dividends.len() && dividends[last].ex_date <= date {
            total += dividends[last].amount;
            if !dividends[last].special {
                regular += dividends[last].amount;
            }
            last += 1;
        }
        while first < last
            && dividends[first].ex_date <= date.checked_sub_months(Months::new(12)).unwrap()
        {
            total -= dividends[first].amount;
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
                dividend_yield: covered.then_some(total.max(0.0) / close),
                dividend_yield_excluding_specials: covered.then_some(regular.max(0.0) / close),
            },
        );
    }
    Ok(Market {
        prices: result,
        dividends,
        splits,
    })
}
