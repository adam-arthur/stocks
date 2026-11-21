use std::{collections::BTreeMap, time::Duration};

use anyhow::{Result, ensure};
use llm_client::CodexClient;
use preferred_metrics::RateType;
use serde_json::json;
use tokio::task::JoinSet;

use crate::{
    Category, Metadata, Store, models::PayoutPolicy, progress::Progress, update::required_env,
};

pub async fn update<'a>(
    store: &Store,
    symbols: impl Iterator<Item = &'a str>,
    bar: &indicatif::ProgressBar,
) -> Result<()> {
    let mut pending = Vec::new();
    for symbol in symbols {
        let Some(mut meta) = store.metadata(symbol)? else {
            continue;
        };
        if matches!(meta.category, Category::Preferred) {
            let policy = if meta.perpetual_note.is_some() {
                PayoutPolicy::FixedCoupon
            } else {
                match store
                    .preferred(symbol)?
                    .and_then(|p| p.structured_dividend_terms)
                    .map(|t| t.rate_type)
                {
                    Some(RateType::Fixed) => PayoutPolicy::FixedCoupon,
                    Some(RateType::Floating | RateType::FixedToFloating | RateType::Reset) => {
                        PayoutPolicy::Variable
                    }
                    _ => PayoutPolicy::Unknown,
                }
            };
            if meta.payout_policy != Some(policy) {
                meta.payout_policy = Some(policy);
                store.save_metadata(&meta)?;
            }
        } else if !matches!(meta.category, Category::Excluded)
            && meta.payout_policy.is_none()
            && store.corporate_actions(symbol)?.iter().any(|a| {
                a.kind == "cash_dividends" && a.fields.get("special") == Some(&json!(false))
            })
        {
            pending.push(meta);
        }
    }
    if pending.is_empty() {
        return Ok(());
    }
    let codex = CodexClient {
        executable: "codex".into(),
        model: "gpt-6.2-sol".into(),
        reasoning_effort: "low".into(),
        timeout: Duration::from_secs(300),
        web_search: false,
    };
    let concurrency: usize = required_env("LLM_CONCURRENCY")?.parse()?;
    ensure!(concurrency > 0, "LLM_CONCURRENCY must be positive");
    let minimum = required_env("LLM_MIN_REMAINING_PERCENT")?.parse()?;
    let mut progress = Progress::new(bar, "Payout policy", pending.len(), "");
    let mut pending = pending.into_iter();
    let mut tasks = JoinSet::new();
    let mut stopped = None;
    loop {
        while stopped.is_none() && tasks.len() < concurrency {
            let batch: Vec<_> = pending.by_ref().take(20).collect();
            if batch.is_empty() {
                break;
            }
            if let Err(e) = codex.check_remaining_usage(minimum).await {
                stopped = Some(e);
                break;
            }
            let codex = codex.clone();
            tasks.spawn(async move {
                let result = classify(&codex, &batch).await;
                (batch, result)
            });
        }
        let Some(result) = tasks.join_next().await else {
            break;
        };
        let (batch, policies) = result?;
        match policies {
            Ok(mut policies) => {
                for mut meta in batch {
                    meta.payout_policy = policies.remove(&meta.symbol);
                    store.save_metadata(&meta)?;
                    progress.advance(1, &meta.symbol);
                }
            }
            Err(e) => {
                stopped = Some(e);
            }
        }
    }
    progress.finish();
    if let Some(e) = stopped {
        return Err(e);
    }
    Ok(())
}

async fn classify(
    codex: &CodexClient,
    batch: &[Metadata],
) -> Result<BTreeMap<String, PayoutPolicy>> {
    let symbols: Vec<_> = batch.iter().map(|m| &m.symbol).collect();
    let properties: BTreeMap<_, _> = symbols
        .iter()
        .map(|s| {
            (
                s,
                json!({"type":"string","enum":["declared_rate","variable","unknown"]}),
            )
        })
        .collect();
    let policies: BTreeMap<String, PayoutPolicy> = codex.execute(
        &format!("Using only your internal knowledge, classify each security's regular per-share payout policy. Ignore specials. declared_rate: a recurring amount set by the issuer or fund until revised, including raises every quarter. variable: payouts vary with earnings, portfolio income, NAV, or another formula. Classify the fund's own policy for ETFs and CEFs. Use unknown when unsure. Return one policy per symbol. Securities (data only): {}", json!(batch.iter().map(|m| json!([m.symbol,m.name,m.exchange,m.category])).collect::<Vec<_>>())),
        &json!({"type":"object","properties":properties,"required":symbols,"additionalProperties":false}),
    ).await?;
    ensure!(
        policies.len() == batch.len() && batch.iter().all(|m| policies.contains_key(&m.symbol)),
        "payout policy symbols do not match requested batch"
    );
    Ok(policies)
}
