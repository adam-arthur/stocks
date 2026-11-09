use std::collections::BTreeMap;

use anyhow::{Context, Result, ensure};
use chrono::NaiveDate;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::market::{self, Market};

#[derive(Deserialize)]
struct Quarter {
    start: NaiveDate,
    end: NaiveDate,
    published_on: Option<NaiveDate>,
    affo_per_share: Option<f64>,
}

pub fn derive(quarters: &[Value], market: &Market, as_of: NaiveDate) -> Result<Value> {
    let mut quarters = quarters
        .iter()
        .cloned()
        .map(serde_json::from_value::<Quarter>)
        .collect::<std::result::Result<Vec<_>, _>>()
        .context("invalid AFFO quarter")?;
    quarters.retain(|quarter| quarter.end <= as_of);
    quarters.sort_by_key(|quarter| (quarter.end, quarter.start));
    ensure!(!quarters.is_empty(), "missing AFFO quarters");
    for quarter in &quarters {
        ensure!(
            (70..=110).contains(&(quarter.end - quarter.start).num_days()),
            "invalid AFFO quarter ending {}",
            quarter.end
        );
    }
    for pair in quarters.windows(2) {
        ensure!(
            pair[0].end.succ_opt() == Some(pair[1].start),
            "missing or overlapping AFFO quarters between {} and {}",
            pair[0].end,
            pair[1].end
        );
    }
    let mut events = Vec::new();
    for window in quarters.windows(4) {
        ensure!(
            (350..=380).contains(&(window[3].end - window[0].start).num_days()),
            "AFFO quarters ending {} do not cover a full year",
            window[3].end
        );
        let mut total = 0.0;
        let mut published = NaiveDate::MIN;
        for quarter in window {
            let date = quarter.published_on.with_context(|| {
                format!(
                    "missing AFFO publication date for quarter ending {}",
                    quarter.end
                )
            })?;
            ensure!(
                date >= quarter.end,
                "AFFO publication date precedes quarter end"
            );
            let value = quarter.affo_per_share.with_context(|| {
                format!("missing affo_per_share for quarter ending {}", quarter.end)
            })?;
            ensure!(value.is_finite(), "non-finite AFFO for {}", quarter.end);
            total += value / market::factor(&market.splits, date, as_of);
            published = published.max(date);
        }
        ensure!(total.is_finite(), "non-finite TTM AFFO");
        events.push((published, window[3].end, total));
    }
    events.sort_by_key(|&(published, end, _)| (published, end));
    let mut events = events.into_iter().peekable();
    let mut current = None;
    let mut previous = None;
    let mut ttm = BTreeMap::new();
    let mut yields = BTreeMap::new();
    for (&date, price) in &market.prices {
        while events
            .peek()
            .is_some_and(|&(published, _, _)| published < date)
        {
            let (_, end, value) = events.next().unwrap();
            if current.is_none_or(|(latest, _)| end > latest) {
                current = Some((end, value));
            }
        }
        if let Some((_, value)) = current {
            if previous != Some(value) {
                ttm.insert(date, value);
                previous = Some(value);
            }
            ensure!(
                (value / price.close).is_finite(),
                "non-finite AFFO yield on {date}"
            );
            yields.insert(date, value / price.close);
        }
    }
    Ok(json!({"affo_per_share_ttm": ttm, "affo_yield": yields}))
}
