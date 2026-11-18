use std::collections::BTreeMap;

use anyhow::{Context, Result, ensure};
use chrono::{NaiveDate, Utc};
use fred_client::{FredClient, TREASURY_SERIES};

use crate::Store;

pub(crate) async fn update(store: &Store) -> Result<()> {
    let client = FredClient::from_env()?;
    let end = Utc::now().date_naive();
    let mut yields = store.treasury()?;
    let mut added = 0;
    for &series in TREASURY_SERIES {
        let history = yields.entry(series.to_owned()).or_default();
        let start = history.last_key_value().map_or_else(
            || NaiveDate::from_ymd_opt(1776, 7, 4).unwrap(),
            |(date, _)| date.succ_opt().unwrap(),
        );
        if start > end {
            continue;
        }
        let new: BTreeMap<_, _> = client
            .observations(series, start, end)
            .await
            .with_context(|| format!("fetching Treasury series {series}"))?
            .into_iter()
            .filter_map(|row| Some((row.date, row.value?)))
            .collect();
        added += new.len();
        history.extend(new);
        ensure!(!history.is_empty(), "no Treasury yields for {series}");
    }
    if added > 0 {
        store.save_treasury(&yields)?;
    }
    log::info!("Treasury cache updated · {added} new yields");
    Ok(())
}
