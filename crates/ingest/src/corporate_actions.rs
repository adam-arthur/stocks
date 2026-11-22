use std::collections::{BTreeMap, BTreeSet};

use crate::{Category, CorporateAction, Store};
use alpaca_client::{AlpacaClient, Asset};
use anyhow::{Context, Result, ensure};
use chrono::{Duration, NaiveDate, Utc};
use indicatif::ProgressBar;
use tokio::task::JoinSet;

use crate::preferred_history;
use crate::progress::Progress;

pub async fn update(
    client: &AlpacaClient,
    store: &Store,
    assets: &[Asset],
    bar: &ProgressBar,
) -> Result<(usize, usize)> {
    let origin = NaiveDate::from_ymd_opt(2016, 1, 1).unwrap();
    let end = Utc::now().date_naive();
    let mut synced = store.corporate_actions_sync()?;
    let mut history_starts = BTreeMap::new();
    let mut groups = BTreeMap::<NaiveDate, Vec<String>>::new();
    for asset in assets {
        if asset.exchange.eq_ignore_ascii_case("OTC")
            || !store
                .metadata(&asset.symbol)?
                .is_some_and(|meta| !matches!(meta.category, Category::Excluded))
        {
            continue;
        }
        let Some(start) = store.prune_market_history(&asset.symbol)? else {
            continue;
        };
        history_starts.insert(asset.symbol.clone(), start);
        groups
            .entry(
                synced
                    .get(&asset.symbol)
                    .map_or(origin, |date| *date - Duration::days(90))
                    .max(start),
            )
            .or_default()
            .push(asset.symbol.clone());
    }
    log::info!("Corporate actions · all types · 90-day overlap · 100 symbols/batch · 4 concurrent");
    let mut progress = Progress::new(bar, "Actions", groups.values().map(Vec::len).sum(), "");
    let mut batches = groups.into_iter().flat_map(|(start, symbols)| {
        symbols
            .chunks(100)
            .map(|chunk| (start, chunk.to_vec()))
            .collect::<Vec<_>>()
    });
    let mut tasks = JoinSet::new();
    let mut updated = 0;
    let mut failed = 0;
    let mut events = 0;
    let mut renames = Vec::new();
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
        let (symbols, result) = result.context("corporate actions task failed")?;
        match result {
            Ok(mut history) => {
                for symbol in &symbols {
                    let mut actions = store
                        .corporate_actions(symbol)?
                        .into_iter()
                        .map(|action| (action.id.clone(), action))
                        .collect::<BTreeMap<_, _>>();
                    for action in history.remove(symbol).unwrap_or_default() {
                        if action.effective_date() < history_starts[symbol] {
                            continue;
                        }
                        events += 1;
                        actions.insert(action.id.clone(), action);
                    }
                    let mut actions = actions.into_values().collect::<Vec<_>>();
                    if actions.iter().any(|a| a.kind == "stock_mergers")
                        && store
                            .metadata(symbol)?
                            .is_some_and(|m| matches!(m.category, Category::Preferred))
                        && let Err(error) = preferred_history::update(
                            client,
                            store,
                            symbol,
                            history_starts[symbol],
                            end,
                            &mut actions,
                        )
                        .await
                    {
                        log::warn!("Predecessor dividend lookup for {symbol} failed: {error:#}");
                        failed += 1;
                    }
                    actions.sort_by(|a, b| (a.process_date, &a.id).cmp(&(b.process_date, &b.id)));
                    if let Some(rename) = ticker_rename(symbol, history_starts[symbol], &actions) {
                        renames.push(rename);
                    }
                    store.save_corporate_actions(symbol, &actions)?;
                    synced.insert(symbol.clone(), end);
                    updated += 1;
                }
                store.save_corporate_actions_sync(&synced)?;
            }
            Err(error) => {
                failed += symbols.len();
                log::warn!(
                    "Corporate actions batch {}–{} failed: {error:#}",
                    symbols.first().unwrap(),
                    symbols.last().unwrap()
                );
            }
        }
        progress.advance(
            symbols.len(),
            &format!("{updated} saved · {events} events fetched · {failed} failed"),
        );
    }
    progress.finish();
    for batch in renames.chunks(100) {
        match backfill_renamed_dividends(client, store, batch, end).await {
            Ok(count) => events += count,
            Err(error) => {
                failed += batch.len();
                log::warn!("Pre-rename dividend lookup failed: {error:#}");
            }
        }
    }
    log::info!("Corporate actions · {events} events fetched or backfilled");
    Ok((updated, failed))
}

struct TickerRename {
    symbol: String,
    previous: String,
    cusip: String,
    start: NaiveDate,
    date: NaiveDate,
}

fn ticker_rename(
    symbol: &str,
    start: NaiveDate,
    actions: &[CorporateAction],
) -> Option<TickerRename> {
    let mut changes = actions.iter().rev().filter(|a| {
        a.kind == "name_changes"
            && a.fields.get("old_symbol") != a.fields.get("new_symbol")
            && ["old_symbol", "new_symbol"]
                .iter()
                .any(|k| a.fields.get(*k).and_then(|v| v.as_str()) == Some(symbol))
    });
    let rename = changes.next()?;
    let field = |key| rename.fields.get(key)?.as_str();
    let cusip = field("new_cusip")?;
    let previous = field("old_symbol")?;
    if field("new_symbol")? != symbol
        || previous.is_empty()
        || cusip.len() != 9
        || field("old_cusip")? != cusip
        || start >= rename.process_date
        || actions.iter().any(|a| {
            a.kind == "cash_dividends"
                && a.fields.get("symbol").and_then(|v| v.as_str()) == Some(symbol)
                && a.effective_date() >= rename.process_date
                && a.fields
                    .get("cusip")
                    .and_then(|v| v.as_str())
                    .is_some_and(|id| !id.is_empty() && id != cusip)
        })
        || changes
            .next()
            .is_some_and(|other| other.process_date == rename.process_date)
    {
        return None;
    }
    Some(TickerRename {
        symbol: symbol.into(),
        previous: previous.into(),
        cusip: cusip.into(),
        start,
        date: rename.process_date,
    })
}

async fn backfill_renamed_dividends(
    client: &AlpacaClient,
    store: &Store,
    batch: &[TickerRename],
    end: NaiveDate,
) -> Result<usize> {
    let cusips = batch
        .iter()
        .map(|r| r.cusip.as_str())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>()
        .join(",");
    let history = fetch_actions(
        client,
        ("cusips", &cusips),
        batch.iter().map(|r| r.start).min().unwrap(),
        end,
    )
    .await?;
    let mut imported = 0;
    for rename in batch {
        let mut actions = store.corporate_actions(&rename.symbol)?;
        let mut changed = false;
        for payment in history.iter().filter(|a| {
            a.kind == "cash_dividends"
                && a.fields.get("cusip").and_then(|v| v.as_str()) == Some(&rename.cusip)
                && a.fields.get("symbol").and_then(|v| v.as_str()) == Some(&rename.previous)
                && a.fields
                    .get("ex_date")
                    .and_then(|v| v.as_str())
                    .and_then(|d| d.parse::<NaiveDate>().ok())
                    .is_some_and(|d| (rename.start..rename.date).contains(&d))
        }) {
            if let Some(existing) = actions.iter_mut().find(|a| a.id == payment.id) {
                if existing.fields == payment.fields
                    && existing.process_date == payment.process_date
                {
                    continue;
                }
                *existing = payment.clone();
            } else if !actions.iter().any(|a| {
                a.kind == payment.kind
                    && a.fields.get("ex_date") == payment.fields.get("ex_date")
                    && a.fields.get("special") == payment.fields.get("special")
            }) {
                actions.push(payment.clone());
            } else {
                continue;
            }
            imported += 1;
            changed = true;
        }
        if changed {
            actions.sort_by(|a, b| (a.process_date, &a.id).cmp(&(b.process_date, &b.id)));
            store.save_corporate_actions(&rename.symbol, &actions)?;
        }
    }
    Ok(imported)
}

async fn fetch(
    client: &AlpacaClient,
    symbols: &[String],
    start: NaiveDate,
    end: NaiveDate,
) -> Result<BTreeMap<String, Vec<CorporateAction>>> {
    let mut history = BTreeMap::<String, Vec<CorporateAction>>::new();
    for action in fetch_actions(client, ("symbols", &symbols.join(",")), start, end).await? {
        let mut related = BTreeSet::new();
        for (key, value) in &action.fields {
            collect_symbols(key, value, &mut related);
        }
        related.retain(|symbol| symbols.iter().any(|requested| requested == symbol));
        ensure!(
            !related.is_empty(),
            "corporate action {} has no requested symbol",
            action.id
        );
        for symbol in related {
            history
                .entry(symbol.to_owned())
                .or_default()
                .push(action.clone());
        }
    }
    Ok(history)
}

pub(crate) async fn fetch_actions(
    client: &AlpacaClient,
    filter: (&str, &str),
    start: NaiveDate,
    end: NaiveDate,
) -> Result<Vec<CorporateAction>> {
    let mut result = Vec::new();
    let mut token = None;
    let mut tokens = BTreeSet::new();
    loop {
        let page = client
            .action_history(
                (filter.0, filter.1.to_owned()),
                start,
                end,
                token.as_deref(),
            )
            .await?;
        for (kind, actions) in page.corporate_actions {
            for mut action in actions {
                action
                    .as_object_mut()
                    .context("invalid corporate action")?
                    .insert("type".into(), kind.clone().into());
                let action: CorporateAction = serde_json::from_value(action)?;
                ensure!(
                    !action.id.is_empty()
                        && action.process_date >= start
                        && action.process_date <= end,
                    "invalid corporate action ID or date"
                );
                result.push(action);
            }
        }
        match page.next_page_token {
            Some(next) => {
                ensure!(tokens.insert(next.clone()), "repeated pagination token");
                token = Some(next);
            }
            None => break,
        }
    }
    Ok(result)
}

fn collect_symbols<'a>(key: &str, value: &'a serde_json::Value, symbols: &mut BTreeSet<&'a str>) {
    match value {
        serde_json::Value::String(symbol) if key == "symbol" || key.ends_with("_symbol") => {
            symbols.insert(symbol);
        }
        serde_json::Value::Object(fields) => {
            for (key, value) in fields {
                collect_symbols(key, value, symbols);
            }
        }
        serde_json::Value::Array(values) => {
            for value in values {
                collect_symbols(key, value, symbols);
            }
        }
        _ => {}
    }
}
