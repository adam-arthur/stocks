use anyhow::{Result, ensure};

#[tokio::main]
async fn main() -> Result<()> {
    let store = ingest::Store::open("data/raw")?;
    let mut symbols: Vec<_> = std::env::args().skip(1).collect();
    if symbols.is_empty() {
        for entry in std::fs::read_dir("data/raw/stocks")? {
            let symbol = entry?.file_name().to_string_lossy().into_owned();
            if store
                .metadata(&symbol)?
                .is_some_and(|meta| matches!(meta.category, ingest::Category::EquityReit))
            {
                symbols.push(symbol);
            }
        }
        symbols.sort();
    }
    let failed =
        ingest::debt::update_symbols(&store, &symbols, &indicatif::ProgressBar::hidden()).await?;
    println!(
        "{} symbols processed; {failed} debt schedules unavailable (data/raw/failed_reit_debt.json)",
        symbols.len()
    );
    ensure!(failed == 0, "{failed} debt schedules unavailable");
    Ok(())
}
