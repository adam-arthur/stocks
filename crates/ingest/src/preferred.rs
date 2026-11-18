use alpaca_client::Asset;
use anyhow::{Context, Result, ensure};
use chrono::Utc;
use llm_client::CodexClient;
use preferred_metrics::PreferredClient;
use preferred_metrics::{DividendTerms, RateType};
use serde_json::json;

use crate::{Category, Failure, Store, progress::Progress};

pub async fn update(
    store: &Store,
    assets: &[Asset],
    progress_bar: &indicatif::ProgressBar,
) -> Result<usize> {
    let mut pending = Vec::new();
    let mut failures = store.preferred_failures()?;
    for asset in assets {
        if store.metadata(&asset.symbol)?.is_some_and(|meta| {
            meta.perpetual_note.is_none() && matches!(meta.category, Category::Preferred)
        }) && store.preferred(&asset.symbol)?.is_none()
        {
            if let Some(failure) = failures
                .iter()
                .find(|failure| failure.symbol == asset.symbol)
            {
                log::info!(
                    "{}: preferred metadata skipped after previous failure: {}",
                    asset.symbol,
                    failure.error
                );
                continue;
            }
            pending.push(&asset.symbol);
        }
    }
    if store.preferred("ATH.PRD")?.is_some() {
        extract_dividend_terms(store, "ATH.PRD").await?;
    }
    if pending.is_empty() {
        log::info!("Preferred metadata · all preferreds already cached or previously failed");
        return Ok(0);
    }
    log::info!(
        "Preferred metadata · {} uncached · preferredstock.ai · sequential requests · 200-ms spacing · no automatic refresh",
        pending.len()
    );
    let mut client = PreferredClient::new()?;
    let mut progress = Progress::new(progress_bar, "Preferred metadata", pending.len(), "");
    let (mut saved, mut failed) = (0, 0);
    for (index, symbol) in pending.iter().enumerate() {
        match client.fetch(symbol).await {
            Ok(preferred) => {
                store.save_preferred(symbol, &preferred)?;
                if symbol.as_str() == "ATH.PRD" {
                    extract_dividend_terms(store, symbol).await?;
                }
                saved += 1;
            }
            Err(error) => {
                failed += 1;
                failures.push(Failure {
                    symbol: (*symbol).clone(),
                    error: format!("{error:#}"),
                    attempted_at: Utc::now(),
                });
                log::warn!("{symbol}: preferred metadata unavailable: {error:#}");
            }
        }
        store.save_preferred_failures(&failures)?;
        progress.advance(1, &format!("{saved} saved · {failed} failed"));
        if client.is_stopped() {
            log::warn!(
                "Preferred metadata paused · {} remaining; saved metadata retained",
                pending.len() - index - 1
            );
            break;
        }
    }
    progress.finish();
    Ok(failed)
}

pub async fn extract_dividend_terms(store: &Store, symbol: &str) -> Result<()> {
    extract_batch(store, &[symbol.to_owned()]).await
}

fn luna() -> CodexClient {
    CodexClient {
        executable: "codex".into(),
        model: "gpt-6-luna".into(),
        reasoning_effort: "medium".into(),
        timeout: std::time::Duration::from_secs(240),
        web_search: false,
    }
}

pub async fn extract_all(store: &Store, symbols: &[String], minimum: f64) -> Result<()> {
    let mut queue: std::collections::VecDeque<_> = symbols.chunks(16).collect();
    let mut failures = Vec::new();
    while let Some(batch) = queue.pop_front() {
        luna().check_remaining_usage(minimum).await?;
        match extract_batch(store, batch).await {
            Ok(()) => log::info!("Processed through {}", batch.last().unwrap()),
            Err(error) if batch.len() > 1 => {
                log::warn!("Retrying smaller batches after {error:#}");
                let (first, second) = batch.split_at(batch.len() / 2);
                queue.push_front(second);
                queue.push_front(first);
            }
            Err(error) => {
                log::warn!("{}: {error:#}", batch[0]);
                failures.push(&batch[0]);
            }
        }
    }
    ensure!(failures.is_empty(), "failed extraction for {failures:?}");
    Ok(())
}

async fn extract_batch(store: &Store, symbols: &[String]) -> Result<()> {
    let mut inputs = Vec::new();
    for symbol in symbols {
        let preferred = store
            .preferred(symbol)?
            .context("missing preferred metadata")?;
        if preferred.structured_dividend_terms.is_some() {
            continue;
        }
        inputs.push(json!({"symbol":symbol, "name":store.metadata(symbol)?.context("missing security metadata")?.name,
            "coupon_text":preferred.dividend_terms, "original_coupon":preferred.original_coupon, "perpetual":preferred.perpetual}));
    }
    if inputs.is_empty() {
        return Ok(());
    }
    #[derive(serde::Deserialize)]
    struct Response {
        rows: Vec<Row>,
    }
    #[derive(serde::Deserialize)]
    struct Row {
        symbol: String,
        rate_type: RateType,
        fixed_annual_coupon: Option<f64>,
    }
    let response: Response = luna().execute(
        &format!("Extract dividend terms independently for every symbol in this JSON array. Use the full security name and coupon text. An explicit fixed-rate designation with matching coupon establishes a fixed rate unless a reset or floating phase is indicated. A bare initial percentage alone is insufficient. Use fixed_annual_coupon as a decimal only for confirmed lifetime fixed rates, otherwise null. Use unknown if ambiguous. Return each symbol exactly once. {}", json!(inputs)),
        &json!({"type":"object","additionalProperties":false,"required":["rows"],"properties":{"rows":{"type":"array","items":{"type":"object","additionalProperties":false,"required":["symbol","rate_type","fixed_annual_coupon"],"properties":{
            "symbol":{"type":"string"},"rate_type":{"type":"string","enum":["fixed","floating","fixed_to_floating","reset","other","unknown"]},
            "fixed_annual_coupon":{"type":["number","null"]}
        }}}}}),
    ).await?;
    let mut expected: std::collections::BTreeSet<_> = inputs
        .iter()
        .filter_map(|row| row["symbol"].as_str())
        .collect();
    let mut checked = Vec::new();
    for row in response.rows {
        let symbol = row.symbol;
        let terms = DividendTerms {
            rate_type: row.rate_type,
            fixed_annual_coupon: row.fixed_annual_coupon,
        };
        ensure!(
            expected.remove(symbol.as_str()),
            "unexpected symbol {symbol}"
        );
        let mut preferred = store
            .preferred(&symbol)?
            .context("missing preferred metadata")?;
        if terms.rate_type == RateType::Fixed {
            let rate = terms.fixed_annual_coupon.context("fixed coupon missing")?;
            ensure!(
                rate > 0.0
                    && rate < 1.0
                    && preferred
                        .original_coupon
                        .is_none_or(|v| (v - rate).abs() < 1e-8),
                "inconsistent fixed coupon for {symbol}"
            );
        } else {
            ensure!(
                terms.fixed_annual_coupon.is_none(),
                "unexpected fixed coupon for {symbol}"
            );
        }
        preferred.structured_dividend_terms = Some(terms);
        checked.push((symbol, preferred));
    }
    ensure!(expected.is_empty(), "missing symbols {expected:?}");
    for (symbol, preferred) in checked {
        store.save_preferred(&symbol, &preferred)?;
    }
    Ok(())
}
