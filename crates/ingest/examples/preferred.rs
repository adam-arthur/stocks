use anyhow::Result;

struct Logger;

impl log::Log for Logger {
    fn enabled(&self, _: &log::Metadata) -> bool {
        true
    }

    fn log(&self, record: &log::Record) {
        eprintln!("{}", record.args());
    }

    fn flush(&self) {}
}

#[tokio::main]
async fn main() -> Result<()> {
    log::set_logger(&Logger).unwrap();
    log::set_max_level(log::LevelFilter::Info);
    let store = ingest::Store::open("data/raw")?;
    let mut assets = alpaca_client::AlpacaClient::from_env()?
        .list_tradable_stocks()
        .await?;
    assets.sort_by(|a, b| a.symbol.cmp(&b.symbol));
    let started = std::time::Instant::now();
    let failed =
        ingest::preferred::update(&store, &assets, &indicatif::ProgressBar::hidden()).await?;
    let (mut total, mut cached, mut unattempted) = (0, 0, 0);
    let failures = store.preferred_failures()?;
    for asset in &assets {
        if store
            .metadata(&asset.symbol)?
            .is_some_and(|meta| matches!(meta.category, ingest::Category::Preferred))
        {
            total += 1;
            if store.preferred(&asset.symbol)?.is_some() {
                cached += 1;
            } else if !failures
                .iter()
                .any(|failure| failure.symbol == asset.symbol)
            {
                unattempted += 1;
            }
        }
    }
    println!(
        "{total} preferreds · {cached} cached · {failed} failed this run · {unattempted} without data or recorded failure · elapsed {:.1}s",
        started.elapsed().as_secs_f64()
    );
    println!("Failures: data/raw/failed_preferred.json");
    Ok(())
}
