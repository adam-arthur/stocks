use anyhow::{Context, Result};

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let symbol = args.next().context("expected symbol")?;
    let path = args.next().context("expected annual filing HTML path")?;
    println!(
        "{}",
        serde_json::to_string_pretty(&reit_metrics::debt::extract(
            &symbol,
            &std::fs::read_to_string(path)?
        )?)?
    );
    Ok(())
}
