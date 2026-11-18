use anyhow::{Context, Result, ensure};

#[tokio::main]
async fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let store = ingest::Store::open(args.next().context("expected data directory")?)?;
    let symbols = args.collect::<Vec<_>>();
    ensure!(!symbols.is_empty(), "expected at least one REIT symbol");
    for symbol in &symbols {
        ensure!(
            store
                .metadata(symbol)?
                .is_some_and(|meta| matches!(meta.category, ingest::Category::EquityReit)),
            "{symbol}: missing equity REIT metadata"
        );
    }
    ingest::reit::update_symbols(&store, &symbols, &indicatif::ProgressBar::hidden()).await?;
    for symbol in &symbols {
        let history = store.reit(symbol)?.context("missing REIT history")?;
        let quarters = &history.quarters;
        println!(
            "{symbol}: {:?}; {} quarters; {} with AFFO/share",
            history.affo_support,
            quarters.len(),
            quarters
                .iter()
                .filter(|quarter| quarter.affo_per_share.is_some())
                .count()
        );
    }
    Ok(())
}
