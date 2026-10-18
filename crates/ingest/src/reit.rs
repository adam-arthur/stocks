use std::collections::BTreeMap;

use alpaca_client::Asset;
use anyhow::Result;
use chrono::NaiveDate;
use reit_metrics::{Fetch, Unit};

use crate::{Category, Store, models::ReitQuarter, progress::Progress};

pub async fn update(
    store: &Store,
    assets: &[Asset],
    progress_bar: &indicatif::ProgressBar,
) -> Result<()> {
    let mut progress = Progress::new(
        progress_bar,
        "REIT financials",
        assets.len(),
        "deterministic",
    );
    for asset in assets {
        if store
            .metadata(&asset.symbol)?
            .is_none_or(|meta| !matches!(meta.category, Category::EquityReit))
        {
            progress.advance(1, "");
            continue;
        }
        let result = async {
            reit_metrics::companies::company(&asset.symbol)?;
            let previous = store.reit(&asset.symbol)?;
            let latest = previous.iter().max_by_key(|quarter| quarter.end);
            let backfill = latest.is_none();
            let report = reit_metrics::fetch(
                &asset.symbol,
                latest.filter(|_| !backfill).map_or(
                    Fetch::All {
                        since: NaiveDate::from_ymd_opt(2016, 1, 1).unwrap(),
                    },
                    |quarter| Fetch::Latest {
                        since: quarter.start,
                    },
                ),
                &store.sec_directory()?.join("reit-metrics"),
            )
            .await?;
            let mut quarters: BTreeMap<_, _> = previous
                .iter()
                .cloned()
                .map(|quarter| ((quarter.start, quarter.end), quarter))
                .collect();
            for value in report
                .observations
                .iter()
                .filter(|value| reit_metrics::scan::is_quarter(value))
            {
                if !backfill && latest.is_some_and(|quarter| value.end < quarter.end) {
                    continue;
                }
                let quarter =
                    quarters
                        .entry((value.start, value.end))
                        .or_insert_with(|| ReitQuarter {
                            start: value.start,
                            end: value.end,
                            ffo_per_share: None,
                            ffo_total: None,
                        });
                quarter.ffo_per_share = (value.unit == Unit::PerShare).then_some(value.value);
                quarter.ffo_total = (value.unit == Unit::Total)
                    .then_some((value.value * value.scale as f64).round() as i64);
            }
            let mut quarters = contiguous_quarters(quarters.into_values().collect());
            if report.report_period.is_some_and(|end| {
                report.observations.iter().all(|value| value.end != end)
                    || quarters.last().is_none_or(|quarter| quarter.end < end)
            }) {
                quarters.clear();
            }
            if serde_json::to_value(&previous)? != serde_json::to_value(&quarters)? {
                store.save_reit(&asset.symbol, &quarters)?;
            }
            Ok::<_, anyhow::Error>(())
        }
        .await;
        if let Err(error) = result {
            log::warn!("{}: REIT metrics unavailable: {error:#}", asset.symbol);
        }
        progress.advance(1, &asset.symbol);
    }
    progress.finish();
    Ok(())
}

pub fn contiguous_quarters(mut quarters: Vec<ReitQuarter>) -> Vec<ReitQuarter> {
    quarters.sort_by_key(|quarter| (quarter.end, quarter.start));
    if quarters
        .last()
        .is_none_or(|quarter| quarter.ffo_per_share.is_none() && quarter.ffo_total.is_none())
    {
        return Vec::new();
    }
    let start = quarters
        .windows(2)
        .rposition(|pair| {
            pair[0].end.succ_opt() != Some(pair[1].start)
                || (pair[0].ffo_per_share.is_none() && pair[0].ffo_total.is_none())
        })
        .map_or(0, |index| index + 1);
    quarters.drain(..start);
    quarters
}
