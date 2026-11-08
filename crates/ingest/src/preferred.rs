use alpaca_client::Asset;
use anyhow::Result;
use chrono::Utc;
use preferred_metrics::PreferredClient;

use crate::{Category, Failure, Store, progress::Progress};

pub async fn update(
    store: &Store,
    assets: &[Asset],
    progress_bar: &indicatif::ProgressBar,
) -> Result<usize> {
    let mut pending = Vec::new();
    let mut failures = store.preferred_failures()?;
    for asset in assets {
        if store
            .metadata(&asset.symbol)?
            .is_some_and(|meta| matches!(meta.category, Category::Preferred))
            && store.preferred(&asset.symbol)?.is_none()
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
