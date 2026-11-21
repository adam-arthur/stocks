use anyhow::{Result, ensure};

mod logging;

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    ensure!(
        args.is_empty() || args.first().is_some_and(|a| a == "--payout-policy") && args.len() > 1,
        "usage: pipeline [--payout-policy SYMBOL ...]"
    );
    match dotenvy::dotenv() {
        Ok(_) => {}
        Err(dotenvy::Error::Io(error)) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    let progress = logging::init();
    let store = ingest::Store::open("data/raw")?;
    if !args.is_empty() {
        return ingest::payout::update(&store, args[1..].iter().map(String::as_str), &progress)
            .await;
    }
    ingest::update(&store, &progress).await
}
