use alpaca_client::Asset;
use anyhow::Result;
use chrono::Utc;
use indicatif::ProgressBar;
use reit_metrics::source::Sources;

use crate::{Category, Failure, Store, progress::Progress};

pub async fn update(store: &Store, assets: &[Asset], bar: &ProgressBar) -> Result<usize> {
    update_symbols(
        store,
        &assets
            .iter()
            .map(|asset| asset.symbol.clone())
            .collect::<Vec<_>>(),
        bar,
    )
    .await
}

pub async fn update_symbols(store: &Store, symbols: &[String], bar: &ProgressBar) -> Result<usize> {
    let sources = Sources::new(&store.sec_directory()?.join("reit-metrics"))?;
    let mut failures = store.debt_failures()?;
    failures.retain(|failure| !symbols.contains(&failure.symbol));
    let mut failed = 0;
    let mut progress = Progress::new(bar, "REIT debt maturities", symbols.len(), "filings");
    for symbol in symbols {
        if store
            .metadata(symbol)?
            .is_some_and(|meta| matches!(meta.category, Category::EquityReit))
        {
            match reit_metrics::debt::fetch(symbol, &sources).await {
                Ok(debt) => store.save_debt(symbol, &debt)?,
                Err(error) => {
                    log::warn!("{}: debt maturities unavailable: {error:#}", symbol);
                    failed += 1;
                    failures.push(Failure {
                        symbol: symbol.clone(),
                        error: format!("{error:#}"),
                        attempted_at: Utc::now(),
                    });
                }
            }
        }
        progress.advance(1, symbol);
    }
    store.save_debt_failures(&failures)?;
    progress.finish();
    Ok(failed)
}
