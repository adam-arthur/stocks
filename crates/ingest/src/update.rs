use std::{
    collections::HashSet,
    env,
    time::{Duration, Instant},
};

use crate::classification::{categorize, classify_batch, is_fresh, save_category};
use crate::{corporate_actions, financials, prices, progress::Progress};

use crate::{Category, Failure, Store};
use alpaca_client::AlpacaClient;
use anyhow::{Context, Result, ensure};
use chrono::Utc;
use llm_client::CodexClient;
use tokio::task::JoinSet;

pub async fn update(store: &Store, progress_bar: &indicatif::ProgressBar) -> Result<()> {
    let started = Instant::now();
    log::info!("Checking usage and loading tradable symbols");
    let codex = CodexClient {
        executable: "codex".into(),
        model: required_env("LLM_MODEL")?,
        reasoning_effort: required_env("LLM_REASONING_EFFORT")?,
        timeout: Duration::from_secs(300),
        web_search: true,
    };
    codex.validate()?;
    let concurrency: usize = required_env("LLM_CONCURRENCY")?
        .parse()
        .context("LLM_CONCURRENCY must be a positive integer")?;
    ensure!(concurrency > 0, "LLM_CONCURRENCY must be greater than zero");
    let minimum: f64 = required_env("LLM_MIN_REMAINING_PERCENT")?
        .parse()
        .context("LLM_MIN_REMAINING_PERCENT must be a number between 0 and 100")?;
    let research_enabled: bool = required_env("LLM_RESEARCH_ENABLED")?
        .parse()
        .context("LLM_RESEARCH_ENABLED must be true or false")?;
    codex.check_remaining_usage(minimum).await?;
    let mut stopped = None;
    let client = AlpacaClient::from_env()?;
    let mut assets = client.list_tradable_stocks().await?;
    assets.sort_by(|a, b| a.symbol.cmp(&b.symbol));
    let mut failures = store.failures()?;
    let mut categorized = 0;
    let mut sec_tickers = None;
    let mut excluded = 0;
    let mut cached = 0;
    let mut skipped_failures = 0;
    let mut failed = 0;
    let mut research = store.pending_research()?;
    let mut unclassified = Vec::new();
    for asset in &assets {
        if store
            .metadata(&asset.symbol)?
            .is_some_and(|meta| is_fresh(meta.updated_at, Utc::now()))
        {
            cached += 1;
            research.retain(|entry| entry.symbol != asset.symbol);
            failures.retain(|entry| entry.symbol != asset.symbol);
            continue;
        }
        if !research.iter().any(|entry| entry.symbol == asset.symbol)
            && !failures.iter().any(|entry| {
                entry.symbol == asset.symbol && is_fresh(entry.attempted_at, Utc::now())
            })
        {
            unclassified.push(asset.clone());
        } else if !research.iter().any(|entry| entry.symbol == asset.symbol) {
            skipped_failures += 1;
        }
    }
    store.save_pending_research(&research)?;
    store.save_failures(&failures)?;
    let quick = CodexClient {
        reasoning_effort: "low".into(),
        web_search: false,
        ..codex.clone()
    };
    let queue_label = if research_enabled {
        "awaiting research"
    } else {
        "deferred (research off)"
    };
    log::info!(
        "Loaded {} tradable symbols · {cached} cached · {skipped_failures} recent failures skipped · {} to classify · {} {queue_label}",
        assets.len(),
        unclassified.len(),
        research.len()
    );
    let mut progress = Progress::new(
        progress_bar,
        "Classify",
        unclassified.len(),
        &format!("{} {queue_label} · 0 new", research.len()),
    );
    let mut completed_batches = 0;
    let mut queued = 0;
    let mut batches = unclassified.chunks(20);
    let mut tasks = JoinSet::new();
    loop {
        while stopped.is_none() && tasks.len() < concurrency {
            let Some(batch) = batches.next() else { break };
            if let Err(error) = codex.check_remaining_usage(minimum).await {
                log::warn!("Stopping new work: {error:#}; saving in-flight results");
                stopped = Some(error);
                break;
            }
            let batch = batch.to_vec();
            let quick = quick.clone();
            tasks.spawn(async move {
                log::debug!("Classifying {} securities from memory", batch.len());
                let result = classify_batch(&quick, &batch).await;
                (batch, result)
            });
        }
        let Some(result) = tasks.join_next().await else {
            break;
        };
        let (batch, result) = result.context("classification task failed")?;
        let batch_size = batch.len();
        if let Err(error) = &result {
            log::warn!("Classification batch failed · {batch_size} symbols unresolved");
            log::debug!("Classification failure: {error:#}");
        }
        for (index, asset) in batch.into_iter().enumerate() {
            match result
                .as_ref()
                .ok()
                .and_then(|result| result.categories[index])
            {
                Some(category) => {
                    save_category(store, &asset, category, &mut sec_tickers).await?;
                    failures.retain(|entry| entry.symbol != asset.symbol);
                    if matches!(category, Category::Excluded) {
                        excluded += 1;
                    } else {
                        categorized += 1;
                    }
                }
                None => {
                    queued += 1;
                    research.push(Failure {
                        symbol: asset.symbol.clone(),
                        error: result.as_ref().err().map_or_else(
                            || "unresolved by first pass".into(),
                            |error| format!("first pass failed: {error:#}"),
                        ),
                        attempted_at: Utc::now(),
                    });
                    log::debug!("{}: {queue_label}", asset.symbol);
                }
            }
        }
        store.save_pending_research(&research)?;
        store.save_failures(&failures)?;
        completed_batches += 1;
        progress.advance(
            batch_size,
            &format!(
                "{completed_batches}/{} batches · {} {queue_label} · {queued} new",
                unclassified.len().div_ceil(20),
                research.len()
            ),
        );
    }
    progress.finish();
    drop(progress);
    if let Some(error) = stopped {
        return Err(error.context("pipeline stopped; completed results saved"));
    }
    let mut unresolved = 0;
    if research_enabled {
        let symbols: HashSet<_> = research.iter().map(|entry| entry.symbol.clone()).collect();
        let pending: Vec<_> = assets
            .iter()
            .filter(|asset| symbols.contains(&asset.symbol))
            .cloned()
            .collect();
        let mut progress = Progress::new(progress_bar, "Research", pending.len(), "");
        let mut pending = pending.into_iter();
        let mut resolved = 0;
        let mut tasks = JoinSet::new();
        loop {
            while stopped.is_none() && tasks.len() < concurrency {
                let Some(asset) = pending.next() else { break };
                if let Err(error) = codex.check_remaining_usage(minimum).await {
                    log::warn!("Stopping new work: {error:#}; saving in-flight results");
                    stopped = Some(error);
                    break;
                }
                let codex = codex.clone();
                tasks.spawn(async move {
                    log::debug!("Researching {} ({})", asset.symbol, asset.name);
                    let result = categorize(&codex, &asset).await;
                    (asset, result)
                });
            }
            let Some(result) = tasks.join_next().await else {
                break;
            };
            let (asset, result) = result.context("categorization task failed")?;
            failures.retain(|failure| failure.symbol != asset.symbol);
            match result {
                Ok(category) => {
                    save_category(store, &asset, category, &mut sec_tickers).await?;
                    if matches!(category, Category::Excluded) {
                        excluded += 1;
                    } else {
                        categorized += 1;
                    }
                    resolved += 1;
                }
                Err(error) => {
                    failed += 1;
                    unresolved += 1;
                    log::warn!("{}: research unresolved", asset.symbol);
                    log::debug!("{}: {error:#}", asset.symbol);
                    failures.push(Failure {
                        symbol: asset.symbol.clone(),
                        error: format!("{error:#}"),
                        attempted_at: Utc::now(),
                    });
                }
            }
            store.save_failures(&failures)?;
            research.retain(|entry| entry.symbol != asset.symbol);
            store.save_pending_research(&research)?;
            progress.advance(1, &format!("{resolved} resolved · {unresolved} unresolved"));
        }
        progress.finish();
        drop(progress);
        if let Some(error) = stopped {
            return Err(error.context("pipeline stopped; completed results saved"));
        }
    } else {
        log::info!(
            "Research disabled · {} symbols retained in the research queue",
            research.len()
        );
    }
    failed += crate::management::update(
        store,
        &assets,
        &CodexClient {
            model: "gpt-6.1-sol".into(),
            reasoning_effort: "low".into(),
            web_search: true,
            ..codex.clone()
        },
        concurrency,
        minimum,
        progress_bar,
    )
    .await?;
    let (prices, price_failures) = prices::update(&client, store, &assets, progress_bar).await?;
    let (actions, action_failures) =
        corporate_actions::update(&client, store, &assets, progress_bar).await?;
    failed += price_failures + action_failures;
    financials::update(store, &assets, progress_bar).await?;
    crate::reit::update(store, &assets, progress_bar).await?;
    log::info!(
        "Done · {categorized} classified · {excluded} excluded · {unresolved} research unresolved · {} {queue_label} · {prices} prices updated · {price_failures} price failures · {actions} corporate action histories updated · {action_failures} corporate action failures · elapsed {}m {}s",
        research.len(),
        started.elapsed().as_secs() / 60,
        started.elapsed().as_secs() % 60
    );
    ensure!(failed == 0, "{failed} updates failed");
    Ok(())
}

fn required_env(name: &str) -> Result<String> {
    let value =
        env::var(name).with_context(|| format!("{name} must be set in .env or the environment"))?;
    ensure!(!value.trim().is_empty(), "{name} must not be empty");
    Ok(value)
}
