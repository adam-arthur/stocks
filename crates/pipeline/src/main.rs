use anyhow::{Result, ensure};

mod logging;

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let refresh = args.first().is_some_and(|a| a == "--refresh-payout-policy");
    let prune = args.first().is_some_and(|a| a == "--prune-market-history");
    ensure!(
        args.is_empty() || (prune || refresh || args[0] == "--payout-policy") && args.len() > 1,
        "usage: pipeline [--payout-policy SYMBOL ... | --refresh-payout-policy SYMBOL ... | --prune-market-history SYMBOL ...]"
    );
    match dotenvy::dotenv() {
        Ok(_) => {}
        Err(dotenvy::Error::Io(error)) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    let progress = logging::init();
    let store = ingest::Store::open("data/raw")?;
    if !args.is_empty() {
        for symbol in &args[1..] {
            store.prune_market_history(symbol)?;
        }
        if prune {
            return Ok(());
        }
        return ingest::payout::update(
            &store,
            args[1..].iter().map(String::as_str),
            &progress,
            refresh,
        )
        .await;
    }
    ingest::update(&store, &progress).await
}
