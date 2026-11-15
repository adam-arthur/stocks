use anyhow::{Context, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let symbol = args.next().context("expected symbol")?;
    let since = args.next().context("expected start date")?.parse()?;
    let cache = args.next().context("expected cache directory")?;
    println!(
        "{}",
        serde_json::to_string_pretty(
            &reit_metrics::fetch(
                &symbol,
                reit_metrics::Fetch::All { since },
                &reit_metrics::source::Sources::new(std::path::Path::new(&cache))?,
            )
            .await?
        )?
    );
    Ok(())
}
