use std::collections::BTreeMap;

use crate::{Category, Store};
use alpaca_client::{AlpacaClient, Asset};
use anyhow::{Context, Result, ensure};
use chrono::{DateTime, Duration, NaiveDate, Utc};
use indicatif::ProgressBar;
use tokio::task::JoinSet;

use crate::progress::Progress;

pub async fn update(
    client: &AlpacaClient,
    store: &Store,
    assets: &[Asset],
    bar: &ProgressBar,
) -> Result<(usize, usize)> {
    if let Some(updated_at) = store.prices_sync()?
        && Utc::now() - updated_at < Duration::hours(6)
    {
        log::info!("Daily closes · skipped · last run {updated_at} was within 6 hours");
        return Ok((0, 0));
    }
    let origin = "2016-01-01T00:00:00Z".parse::<DateTime<Utc>>()?;
    let end = (Utc::now()
        .date_naive()
        .and_hms_opt(0, 0, 0)
        .unwrap()
        .and_utc()
        - Duration::seconds(1))
    .min(Utc::now() - Duration::minutes(15));
    let mut groups = BTreeMap::<DateTime<Utc>, Vec<String>>::new();
    let mut otc = 0;
    for asset in assets {
        if !store
            .metadata(&asset.symbol)?
            .is_some_and(|meta| !matches!(meta.category, Category::Excluded))
        {
            continue;
        }
        if asset.exchange.eq_ignore_ascii_case("OTC") {
            otc += 1;
            continue;
        }
        let start = store
            .prices(&asset.symbol)?
            .last_key_value()
            .map_or(origin, |(date, _)| {
                date.and_hms_opt(0, 0, 0).unwrap().and_utc()
            });
        if start <= end {
            groups.entry(start).or_default().push(asset.symbol.clone());
        }
    }
    let total = groups.values().map(Vec::len).sum();
    log::info!(
        "Daily closes · SIP, unadjusted · {otc} OTC skipped · 100 symbols/batch · 4 concurrent · at most 200 requests/min"
    );
    let mut progress = Progress::new(bar, "Prices", total, "");
    let mut batches = groups.into_iter().flat_map(|(start, symbols)| {
        symbols
            .chunks(100)
            .map(|chunk| (start, chunk.to_vec()))
            .collect::<Vec<_>>()
    });
    let mut tasks = JoinSet::new();
    let mut updated = 0;
    let mut empty = 0;
    let mut failed = 0;
    let mut points = 0;
    loop {
        while tasks.len() < 4 {
            let Some((start, symbols)) = batches.next() else {
                break;
            };
            let client = client.clone();
            tasks.spawn(async move {
                let result = fetch(&client, &symbols, start, end).await;
                (symbols, result)
            });
        }
        let Some(result) = tasks.join_next().await else {
            break;
        };
        let (symbols, result) = result.context("price task failed")?;
        match result {
            Ok(mut history) => {
                for symbol in &symbols {
                    if let Some(new) = history.remove(symbol).filter(|bars| !bars.is_empty()) {
                        points += new.len();
                        let mut prices = store.prices(symbol)?;
                        prices.extend(new);
                        store.save_prices(symbol, &prices)?;
                        updated += 1;
                    } else {
                        empty += 1;
                        log::debug!("{symbol}: no daily bars returned");
                    }
                }
            }
            Err(error) => {
                failed += symbols.len();
                log::warn!(
                    "Price batch {}–{} failed: {error:#}",
                    symbols.first().unwrap(),
                    symbols.last().unwrap()
                );
            }
        }
        progress.advance(symbols.len(), &format!("{updated} saved · {points} bars fetched · {empty} without new bars · {failed} failed"));
    }
    progress.finish();
    store.save_prices_sync(Utc::now())?;
    Ok((updated, failed))
}

async fn fetch(
    client: &AlpacaClient,
    symbols: &[String],
    start: DateTime<Utc>,
    end: DateTime<Utc>,
) -> Result<BTreeMap<String, BTreeMap<NaiveDate, f64>>> {
    let mut history = BTreeMap::<String, BTreeMap<NaiveDate, f64>>::new();
    let mut token = None;
    loop {
        let page = client
            .daily_bars(symbols, start, end, token.as_deref())
            .await?;
        for (symbol, bars) in page.bars {
            ensure!(symbols.contains(&symbol), "unexpected symbol {symbol}");
            for bar in bars {
                ensure!(
                    bar.c.is_finite() && bar.c > 0.0 && bar.t >= start && bar.t <= end,
                    "invalid daily bar for {symbol}"
                );
                history
                    .entry(symbol.clone())
                    .or_default()
                    .insert(bar.t.date_naive(), bar.c);
            }
        }
        match page.next_page_token {
            Some(next) => {
                ensure!(token.as_ref() != Some(&next), "repeated pagination token");
                token = Some(next);
            }
            None => break,
        }
    }
    Ok(history)
}
