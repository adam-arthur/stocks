use std::{
    collections::{BTreeMap, BTreeSet},
    time::{Duration, Instant},
};

use alpaca_client::Asset;
use anyhow::{Context, Result, ensure};
use chrono::{NaiveDate, Utc};
use indicatif::ProgressBar;
use sec_client::{SecClient, retain_periods_since};

use crate::{Category, Store, models::FinancialsSync, progress::Progress};

pub async fn update(store: &Store, assets: &[Asset], bar: &ProgressBar) -> Result<()> {
    let lock = std::fs::File::options()
        .create(true)
        .append(true)
        .open(store.sec_directory()?.join("sync.lock"))?;
    lock.try_lock()
        .context("acquiring financials refresh lock")?;
    let mut symbols = BTreeMap::new();
    for asset in assets {
        if let Some(meta) = store.metadata(&asset.symbol)?.filter(|meta| {
            matches!(
                meta.category,
                Category::Stock
                    | Category::EquityReit
                    | Category::MortgageReit
                    | Category::Bdc
                    | Category::AssetManager
            )
        }) {
            symbols.insert(asset.symbol.clone(), meta.cik);
        }
    }
    if symbols.is_empty() {
        log::info!("Financials · no eligible companies");
        return Ok(());
    }
    let companies: BTreeSet<_> = symbols.values().flatten().copied().collect();
    let today = Utc::now().date_naive();
    let previous = store.financials_sync()?;
    if previous.as_ref().is_some_and(|sync| {
        sync.updated_on == today
            && sync.companies == companies
            && companies.iter().all(|cik| {
                sync.unavailable.contains(cik)
                    || (sync.fingerprints.contains_key(cik) && store.has_company_facts(*cik))
            })
    }) {
        log::info!("Financials · already refreshed today");
        return Ok(());
    }
    let mut sync = FinancialsSync {
        updated_on: today,
        companies: BTreeSet::new(),
        fingerprints: BTreeMap::new(),
        unavailable: Default::default(),
    };
    let unmapped = symbols.values().filter(|cik| cik.is_none()).count();
    log::info!(
        "Financials · {} companies · {unmapped} symbols without SEC mappings · periods ending since 2016-01-01",
        companies.len()
    );
    if companies.is_empty() {
        store.save_financials_sync(&sync)?;
        return Ok(());
    }
    log::info!("Downloading SEC Company Facts bulk archive");
    let mut last_log = Instant::now();
    let mut archive = SecClient::from_env()?
        .company_facts_bulk(&store.sec_directory()?, |downloaded, total| {
            if last_log.elapsed() >= Duration::from_secs(10) || total == Some(downloaded) {
                log::info!(
                    "SEC archive · {:.0} MB downloaded{}",
                    downloaded as f64 / 1_000_000.0,
                    total.map_or(String::new(), |total| format!(
                        " of {:.0} MB",
                        total as f64 / 1_000_000.0
                    ))
                );
                last_log = Instant::now();
            }
        })
        .await?;
    store.save_financials_sync(&sync)?;
    sync.companies = companies.clone();
    let mut progress = Progress::new(bar, "Financials", symbols.len(), "");
    progress.advance(unmapped, &format!("{unmapped} unmapped"));
    let mut saved = 0;
    let mut unchanged = 0;
    let mut failed = 0;
    for cik in companies {
        let result: Result<_> = (|| {
            let Some(fingerprint) = archive.fingerprint(cik)? else {
                return Ok(None);
            };
            let cached = previous
                .as_ref()
                .and_then(|sync| sync.fingerprints.get(&cik))
                == Some(&fingerprint)
                && store.has_company_facts(cik);
            if !cached {
                let mut facts = archive
                    .company_facts(cik)?
                    .context("missing SEC archive entry")?;
                retain_periods_since(&mut facts, NaiveDate::from_ymd_opt(2016, 1, 1).unwrap())?;
                store.save_company_facts(cik, &facts)?;
            }
            Ok(Some((fingerprint, cached)))
        })();
        match result {
            Ok(Some((fingerprint, cached))) => {
                sync.fingerprints.insert(cik, fingerprint);
                if cached {
                    unchanged += 1;
                } else {
                    saved += 1;
                }
            }
            Ok(None) => {
                sync.unavailable.insert(cik);
                log::debug!("CIK{cik:010}: absent from SEC Company Facts archive");
            }
            Err(error) => {
                failed += 1;
                log::warn!("CIK{cik:010}: financials failed: {error:#}");
            }
        }
        progress.advance(
            symbols
                .values()
                .filter(|value| **value == Some(cik))
                .count(),
            &format!(
                "{saved} companies saved · {unchanged} unchanged · {} unavailable · {failed} failed",
                sync.unavailable.len()
            ),
        );
    }
    progress.finish();
    store.save_financials_sync(&sync)?;
    ensure!(
        failed == 0,
        "financials: {failed} companies failed; daily refresh not marked complete"
    );
    Ok(())
}
