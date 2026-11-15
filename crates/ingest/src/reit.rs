use std::collections::BTreeMap;

use alpaca_client::Asset;
use anyhow::Result;
use chrono::NaiveDate;
use reit_metrics::{Fetch, Metric, Unit, source::Sources};

use crate::{Category, Store, models::ReitQuarter, progress::Progress};

pub async fn update(
    store: &Store,
    assets: &[Asset],
    progress_bar: &indicatif::ProgressBar,
) -> Result<()> {
    update_symbols(
        store,
        &assets
            .iter()
            .map(|asset| asset.symbol.clone())
            .collect::<Vec<_>>(),
        progress_bar,
    )
    .await?;
    Ok(())
}

pub async fn update_symbols(
    store: &Store,
    symbols: &[String],
    progress_bar: &indicatif::ProgressBar,
) -> Result<usize> {
    let mut failed = 0;
    let sources = Sources::new(&store.sec_directory()?.join("reit-metrics"))?;
    let mut progress = Progress::new(
        progress_bar,
        "REIT financials",
        symbols.len(),
        "deterministic",
    );
    for symbol in symbols {
        if store
            .metadata(symbol)?
            .is_none_or(|meta| !matches!(meta.category, Category::EquityReit))
        {
            progress.advance(1, "");
            continue;
        }
        let result = async {
            let company = reit_metrics::companies::company(symbol)?;
            let history = store.reit(symbol)?;
            let previous = history
                .as_ref()
                .map_or(&[][..], |history| history.quarters.as_slice());
            let latest = previous.iter().max_by_key(|quarter| quarter.end);
            let mut backfill = latest.is_none_or(|latest| {
                previous.iter().any(|quarter| {
                    quarter.published_on.is_none()
                        || quarter.ffo_per_share.is_some() != latest.ffo_per_share.is_some()
                        || quarter.ffo_total.is_some() != latest.ffo_total.is_some()
                })
            });
            let ownership_backfill = reit_metrics::ownership::supports(symbol)
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
                symbol,
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
                    symbol,
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
                            published_on: None,
                            ffo_per_share: None,
                            ffo_total: None,
                            affo_per_share: None,
                            affo_total: None,
                            ownership: Default::default(),
                        });
                anyhow::ensure!(
                    quarter.published_on.is_none() || quarter.published_on == value.published_on,
                    "conflicting publication dates for quarter ending {}",
                    value.end
                );
                quarter.published_on = value.published_on;
                match (value.metric, value.unit) {
                    (Metric::Ffo, Unit::PerShare) => quarter.ffo_per_share = Some(value.value),
                    (Metric::Ffo, Unit::Total) => {
                        quarter.ffo_total = Some((value.value * value.scale as f64).round() as i64);
                    }
                    (Metric::Affo, Unit::PerShare) => quarter.affo_per_share = Some(value.value),
                    (Metric::Affo, Unit::Total) => {
                        quarter.affo_total =
                            Some((value.value * value.scale as f64).round() as i64);
                    }
                }
            }
            let facts = store
                .metadata(symbol)?
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
            let updated = reit_metrics::ReitHistory {
                affo_support: company.affo_support,
                quarters,
            };
            if serde_json::to_value(&history)? != serde_json::to_value(&updated)? {
                store.save_reit(symbol, &updated)?;
            }
            Ok::<_, anyhow::Error>(())
        }
        .await;
        if let Err(error) = result {
            failed += 1;
            log::warn!("{}: REIT metrics unavailable: {error:#}", symbol);
        }
        progress.advance(1, symbol);
    }
    progress.finish();
    Ok(failed)
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
