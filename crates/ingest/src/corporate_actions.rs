use std::collections::{BTreeMap, BTreeSet};

use crate::{Category, CorporateAction, Store};
use alpaca_client::{AlpacaClient, Asset};
use anyhow::{Context, Result, ensure};
use chrono::{Duration, NaiveDate, Utc};
use indicatif::ProgressBar;
use tokio::task::JoinSet;

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
    let mut groups = BTreeMap::<NaiveDate, Vec<String>>::new();
    for asset in assets {
        if asset.exchange.eq_ignore_ascii_case("OTC")
            || !store
                .metadata(&asset.symbol)?
                .is_some_and(|meta| !matches!(meta.category, Category::Excluded))
        {
            continue;
        }
        groups
            .entry(
                synced
                    .get(&asset.symbol)
                    .map_or(origin, |date| (*date - Duration::days(90)).max(origin)),
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
                        events += 1;
                        actions.insert(action.id.clone(), action);
                    }
                    let mut actions = actions.into_values().collect::<Vec<_>>();
                    actions.sort_by(|a, b| (a.process_date, &a.id).cmp(&(b.process_date, &b.id)));
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
    Ok((updated, failed))
}

async fn fetch(
    client: &AlpacaClient,
    symbols: &[String],
    start: NaiveDate,
    end: NaiveDate,
) -> Result<BTreeMap<String, Vec<CorporateAction>>> {
    let mut history = BTreeMap::<String, Vec<CorporateAction>>::new();
    let mut token = None;
    let mut tokens = BTreeSet::new();
    loop {
        let page = client
            .corporate_actions(symbols, start, end, token.as_deref())
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
        }
        match page.next_page_token {
            Some(next) => {
                ensure!(tokens.insert(next.clone()), "repeated pagination token");
                token = Some(next);
            }
            None => break,
        }
    }
    Ok(history)
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
