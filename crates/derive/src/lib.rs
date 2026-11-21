mod debt;
mod dividend_stats;
mod financials;
mod market;
mod output;
mod preferred_spread;
mod quarters;
mod reit;
mod valuation;
mod yield_stats;
pub use yield_stats::refresh as refresh_yield_stats;

use std::{collections::BTreeMap, fs, io::Write, path::Path};

use anyhow::{Context, Result, ensure};
use chrono::{NaiveDate, Utc};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

fn lock_output(output: &Path) -> Result<fs::File> {
    let lock = fs::File::options()
        .create(true)
        .truncate(false)
        .write(true)
        .open(output.join(".lock"))?;
    lock.try_lock()
        .context("another derive process is using this output directory")?;
    Ok(lock)
}

pub fn run(input: &Path, output: &Path) -> Result<usize> {
    ensure!(input.is_dir(), "input directory does not exist");
    fs::create_dir_all(output)?;
    ensure!(
        fs::canonicalize(input)? != fs::canonicalize(output)?,
        "output must differ from ingest directory"
    );
    let _lock = lock_output(output)?;
    let input_lock = fs::File::options().read(true).open(input.join(".lock"))?;
    input_lock
        .try_lock_shared()
        .context("ingest is running; derive requires a consistent input snapshot")?;
    if input.join("treasury.json").try_exists()? {
        let mut file = tempfile::NamedTempFile::new_in(output)?;
        std::io::copy(&mut fs::File::open(input.join("treasury.json"))?, &mut file)?;
        file.persist(output.join("treasury.json"))?;
    }
    fs::create_dir_all(output.join("stocks"))?;
    let as_of = Utc::now().date_naive();
    let synced = read::<BTreeMap<String, NaiveDate>>(&input.join("corporate_actions_sync.json"))?
        .unwrap_or_default();
    let mut metadata = Vec::new();
    for entry in fs::read_dir(input.join("stocks"))? {
        let entry = entry?;
        if entry.file_type()?.is_dir()
            && let Some(meta) = read::<Value>(&entry.path().join("meta.json"))?
        {
            ensure!(
                meta["symbol"].as_str() == entry.file_name().to_str(),
                "metadata symbol does not match directory"
            );
            metadata.push(meta);
        }
    }
    metadata.sort_by(|a, b| a["symbol"].as_str().cmp(&b["symbol"].as_str()));
    let mut companies = BTreeMap::<u64, usize>::new();
    for meta in metadata.iter().filter(|m| common(m)) {
        if let Some(cik) = meta["cik"].as_u64() {
            *companies.entry(cik).or_default() += 1;
        }
    }
    let treasury =
        read::<preferred_spread::Treasury>(&input.join("treasury.json"))?.unwrap_or_default();
    let mut saved = 0;
    let mut failures = BTreeMap::new();
    for meta in metadata.iter().filter(|m| m["category"] != "EXCLUDED") {
        let symbol = meta["symbol"].as_str().context("metadata missing symbol")?;
        match security(
            input,
            meta,
            &companies,
            synced.get(symbol).copied(),
            as_of,
            &treasury,
        ) {
            Ok(mut data) => {
                let directory = output.join("stocks").join(symbol);
                fs::create_dir_all(&directory)?;
                for history in ["prices", "dividends", "financials"] {
                    write(
                        &directory.join(format!("{history}.json")),
                        &data.as_object_mut().unwrap().remove(history).unwrap(),
                    )?;
                }
                for name in ["valuation", "preferred_spread"] {
                    let path = directory.join(format!("{name}.json"));
                    if let Some(history) = data.as_object_mut().unwrap().remove(name) {
                        write(&path, &history)?;
                    } else {
                        match fs::remove_file(path) {
                            Ok(()) => {}
                            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                            Err(error) => return Err(error.into()),
                        }
                    }
                }
                write(&directory.join("meta.json"), &data)?;
                saved += 1;
            }
            Err(error) => {
                eprintln!("{symbol}: {error:#}");
                failures.insert(symbol, format!("{error:#}"));
            }
        }
        if (saved + failures.len()) % 500 == 0 {
            eprintln!("Derived {} securities; {} failed", saved, failures.len());
        }
    }
    write(
        &output.join("failures.json"),
        &serde_json::to_value(&failures)?,
    )?;
    ensure!(
        failures.is_empty(),
        "{} securities failed; see failures.json",
        failures.len()
    );
    Ok(saved)
}

fn common(meta: &Value) -> bool {
    matches!(
        meta["category"].as_str(),
        Some("STOCK" | "EQUITY_REIT" | "MORTGAGE_REIT" | "BDC" | "ASSET_MANAGER")
    )
}

fn security(
    input: &Path,
    meta: &Value,
    companies: &BTreeMap<u64, usize>,
    synced: Option<NaiveDate>,
    as_of: NaiveDate,
    treasury: &preferred_spread::Treasury,
) -> Result<Value> {
    let symbol = meta["symbol"].as_str().unwrap();
    let mut market = market::derive(
        read(&input.join("stocks").join(symbol).join("prices.json"))?.unwrap_or_default(),
        &read::<Vec<Value>>(
            &input
                .join("stocks")
                .join(symbol)
                .join("corporate_actions.json"),
        )?
        .unwrap_or_default(),
        synced,
        as_of,
    )?;
    let is_preferred = meta["category"] == "PREFERRED";
    if is_preferred {
        for price in market.prices.values_mut() {
            for value in [&mut price.dividend_yield_ttm, &mut price.dividend_yield_fwd] {
                *value = value
                    .filter(|_| meta["perpetual_note"].is_null())
                    .map(output::round_yield);
            }
        }
    }
    let facts = if common(meta) {
        meta["cik"]
            .as_u64()
            .map(|cik| read::<Value>(&input.join(format!("sec/company_facts/CIK{cik:010}.json"))))
            .transpose()?
            .flatten()
    } else {
        None
    };
    let extracted = ::financials::extract(symbol, facts.as_ref().unwrap_or(&Value::Null), as_of);
    if meta["category"] == "EQUITY_REIT" {
        for issue in &extracted.issues {
            eprintln!(
                "{symbol}: {} at {}: {}",
                issue.metric, issue.end, issue.problem
            );
        }
    }
    let mut financials = extracted.financials;
    financials::calculate(&mut financials);
    let mut summary = json!({"latest_quarter_end": null, "closing_price": null, "market_cap": null,
        "shares_outstanding": null, "dividend_yield_ttm": null, "dividend_yield_fwd": null,
        "revenue_ttm": null, "earnings_ttm": null, "common_shareholder_earnings_ttm": null,
        "diluted_eps": null, "price_to_earnings": null, "price_to_sales": null, "price_to_book": null,
        "total_assets": null, "cash_and_equivalents": null, "total_debt": null, "preferred_stock_value": null, "total_debt_including_preferred_stock": null,
        "common_shareholder_equity": null, "net_debt_to_assets": null});
    let date = market
        .prices
        .last_key_value()
        .map_or(as_of, |(date, _)| *date);
    summary["latest_quarter_end"] = json!(
        financials
            .values()
            .flatten()
            .filter(|o| o.start.is_some() && o.end <= date && o.filed <= date)
            .map(|o| o.end)
            .max()
    );
    if let Some((_, price)) = market.prices.last_key_value() {
        summary["closing_price"] = json!(price.close);
        summary["dividend_yield_ttm"] = json!(price.dividend_yield_ttm);
        summary["dividend_yield_fwd"] = json!(price.dividend_yield_fwd);
        let ambiguous = meta["cik"]
            .as_u64()
            .is_some_and(|cik| companies.get(&cik).is_some_and(|count| *count > 1))
            || meta["name"].as_str().is_some_and(|name| {
                ["class ", "depositary", "depository"]
                    .iter()
                    .any(|term| name.to_ascii_lowercase().contains(term))
            });
        if !ambiguous
            && common(meta)
            && let Some(shares) = financials::latest(&financials, "shares_outstanding", date)
                .filter(|o| o.value > 0.0)
        {
            summary["shares_outstanding"] = json!(shares.value);
            summary["market_cap"] = json!(
                price.close * shares.value * market::factor(&market.splits, shares.end, as_of)
            );
        }
    }
    for metric in ["revenue", "earnings", "common_shareholder_earnings"] {
        summary[format!("{metric}_ttm")] =
            json!(financials::trailing(&financials, metric, date).map(|o| o.value));
    }
    for metric in [
        "total_assets",
        "cash_and_equivalents",
        "total_debt",
        "preferred_stock_value",
        "total_debt_including_preferred_stock",
        "common_shareholder_equity",
    ] {
        summary[metric] = json!(
            financials::latest(&financials, metric, date)
                .filter(|o| Some(o.end)
                    == financials
                        .values()
                        .flatten()
                        .filter(|o| o.start.is_none()
                            && o.unit == "USD"
                            && o.end <= date
                            && o.filed <= date)
                        .map(|o| o.end)
                        .max())
                .map(|o| o.value)
        );
    }
    summary["net_debt_to_assets"] = json!(
        summary["total_debt"]
            .as_f64()
            .zip(summary["cash_and_equivalents"].as_f64())
            .zip(summary["total_assets"].as_f64())
            .filter(|(_, assets)| *assets > 0.0)
            .map(|((debt, cash), assets)| (debt - cash) / assets)
    );
    summary["diluted_eps"] = json!(
        financials
            .get("diluted_earnings_per_share")
            .into_iter()
            .flatten()
            .filter(|o| o.end <= date
                && o.filed <= date
                && o.start
                    .is_some_and(|start| (70..=110).contains(&(o.end - start).num_days())))
            .max_by_key(|o| (o.end, o.filed, o.start))
            .map(|o| o.value)
    );
    for (ratio, metric) in [
        ("price_to_earnings", "common_shareholder_earnings_ttm"),
        ("price_to_sales", "revenue_ttm"),
        ("price_to_book", "common_shareholder_equity"),
    ] {
        summary[ratio] = json!(
            summary["market_cap"]
                .as_f64()
                .zip(summary[metric].as_f64())
                .filter(|(_, denominator)| *denominator > 0.0)
                .map(|(cap, denominator)| cap / denominator)
        );
    }
    output::round_summary(&mut summary, is_preferred);
    let mut quarters = quarters::group(&financials);
    let mut valuation = None;
    let mut affo_support = None;
    if meta["category"] == "EQUITY_REIT" {
        let history = read::<reit_metrics::ReitHistory<Value>>(
            &input.join("stocks").join(symbol).join("reit.json"),
        )?;
        affo_support = history.as_ref().map(|history| history.affo_support);
        let reit = history.map(|history| history.quarters).unwrap_or_default();
        if affo_support.is_some_and(reit_metrics::AffoSupport::is_supported)
            && reit.windows(4).any(|window| {
                window
                    .iter()
                    .all(|quarter| quarter["affo_per_share"].is_number())
            })
        {
            valuation = Some(
                valuation::derive(&reit, &market, as_of)
                    .with_context(|| format!("{symbol}: AFFO valuation"))?,
            );
        }
        reit::merge(&mut quarters, reit, symbol != "MKZR");
    }
    let mut result = json!({
        "metadata": meta,
        "as_of": as_of,
        "prices": market.prices,
        "dividends": market.dividends,
        "financials": quarters,
        "summary": summary,
        "debt": if meta["category"] == "EQUITY_REIT" {
            debt::load(&input.join("stocks").join(symbol).join("debt.json"))?
        } else {
            None
        },
        "preferred": if is_preferred {
            read::<Value>(&input.join("stocks").join(symbol).join("preferred.json"))?
        } else {
            None
        },
    });
    if let Some((history, snapshot)) = preferred_spread::derive(
        &result["preferred"],
        &meta["perpetual_note"],
        &market,
        treasury,
    ) {
        result["preferred_spread"] = history;
        result["summary"]["preferred_spread"] = snapshot;
    }
    if let Some(preferred) = result["preferred"].as_object_mut()
        && let Some(terms) = preferred.remove("structured_dividend_terms")
    {
        preferred.insert("dividend_terms".into(), terms);
    }
    dividend_stats::update(&mut result, &market.prices, &market.dividends, as_of);
    yield_stats::update(&mut result, &market.prices, treasury, as_of);
    if let Some(affo_support) = affo_support {
        result["affo_support"] = json!(affo_support);
    }
    if let Some(valuation) = valuation {
        result["valuation"] = valuation;
    }
    Ok(result)
}

fn read<T: DeserializeOwned>(path: &Path) -> Result<Option<T>> {
    match fs::read(path) {
        Ok(bytes) => {
            Ok(Some(serde_json::from_slice(&bytes).with_context(|| {
                format!("invalid JSON in {}", path.display())
            })?))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

fn write(path: &Path, value: &Value) -> Result<()> {
    let mut file = tempfile::NamedTempFile::new_in(path.parent().unwrap())?;
    {
        let mut writer = std::io::BufWriter::new(&mut file);
        serde_json::to_writer_pretty(&mut writer, &output::Document(value))?;
        writer.flush()?;
    }
    file.write_all(b"\n")?;
    file.as_file().sync_all()?;
    file.persist(path)?;
    Ok(())
}
