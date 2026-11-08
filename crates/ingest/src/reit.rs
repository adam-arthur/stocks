use std::collections::BTreeMap;

use alpaca_client::Asset;
use anyhow::Result;
use chrono::NaiveDate;
use reit_metrics::{Fetch, Unit, source::Sources};

use crate::{Category, Store, models::ReitQuarter, progress::Progress};

pub async fn update(
    store: &Store,
    assets: &[Asset],
    progress_bar: &indicatif::ProgressBar,
) -> Result<()> {
    let sources = Sources::new(&store.sec_directory()?.join("reit-metrics"))?;
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
            let mut backfill = latest.is_none_or(|latest| {
                previous.iter().any(|quarter| {
                    quarter.ffo_per_share.is_some() != latest.ffo_per_share.is_some()
                        || quarter.ffo_total.is_some() != latest.ffo_total.is_some()
                })
            });
            let ownership_backfill = reit_metrics::ownership::supports(&asset.symbol)
                && [false, true].into_iter().any(|equivalents| {
                    let present = |quarter: &ReitQuarter| {
                        quarter.ownership.as_ref().is_some_and(|ownership| {
                            if equivalents {
                                ownership.outside_op_unit_share_equivalents.is_some()
                            } else {
                                ownership.outside_op_units.is_some()
                            }
                        })
                    };
                    previous
                        .iter()
                        .filter(|quarter| present(quarter))
                        .map(|quarter| quarter.end)
                        .min()
                        .is_some_and(|start| {
                            previous
                                .iter()
                                .any(|quarter| quarter.end >= start && !present(quarter))
                        })
                });
            let mut report = reit_metrics::fetch(
                &asset.symbol,
                latest.filter(|_| !backfill && !ownership_backfill).map_or(
                    Fetch::All {
                        since: NaiveDate::from_ymd_opt(2016, 1, 1).unwrap(),
                    },
                    |quarter| Fetch::Latest {
                        since: quarter.start,
                        unit: if quarter.ffo_per_share.is_some() {
                            Unit::PerShare
                        } else {
                            Unit::Total
                        },
                    },
                ),
                &sources,
            )
            .await?;
            if !backfill && report.observations.is_empty() {
                backfill = true;
                report = reit_metrics::fetch(
                    &asset.symbol,
                    Fetch::All {
                        since: NaiveDate::from_ymd_opt(2016, 1, 1).unwrap(),
                    },
                    &sources,
                )
                .await?;
            }
            let mut quarters: BTreeMap<_, _> = previous
                .iter()
                .filter(|_| !backfill)
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
                            ownership: Default::default(),
                        });
                quarter.ffo_per_share = (value.unit == Unit::PerShare).then_some(value.value);
                quarter.ffo_total = (value.unit == Unit::Total)
                    .then_some((value.value * value.scale as f64).round() as i64);
            }
            let facts = store
                .metadata(&asset.symbol)?
                .and_then(|meta| meta.cik)
                .map(|cik| store.company_facts(cik))
                .transpose()?
                .flatten();
            for quarter in quarters.values_mut() {
                let mut ownership = report.ownership.remove(&quarter.end).unwrap_or_default();
                if let Some(shares) = facts.as_ref().and_then(|facts| {
                    reit_metrics::ownership::common_shares(
                        facts,
                        quarter.end,
                        chrono::Utc::now().date_naive(),
                    )
                }) {
                    reit_metrics::ownership::merge(
                        &mut ownership,
                        reit_metrics::ownership::Ownership {
                            shares_outstanding: Some(shares),
                            ..Default::default()
                        },
                    );
                }
                if ownership != Default::default() {
                    quarter
                        .ownership
                        .get_or_insert_with(Default::default)
                        .update(ownership);
                }
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
                || pair[0].ffo_per_share.is_some() != pair[1].ffo_per_share.is_some()
                || pair[0].ffo_total.is_some() != pair[1].ffo_total.is_some()
        })
        .map_or(0, |index| index + 1);
    quarters.drain(..start);
    let mut contiguous = [true; 2];
    for quarter in quarters.iter_mut().rev() {
        if let Some(ownership) = &mut quarter.ownership {
            for (present, value) in contiguous.iter_mut().zip([
                &mut ownership.outside_op_units,
                &mut ownership.outside_op_unit_share_equivalents,
            ]) {
                *present &= value.is_some();
                if !*present {
                    *value = None;
                }
            }
            if ownership.shares_outstanding.is_none()
                && ownership.outside_op_units.is_none()
                && ownership.outside_op_unit_share_equivalents.is_none()
            {
                quarter.ownership = None;
            }
        } else {
            contiguous = [false; 2];
        }
    }
    quarters
}
