use anyhow::{Context, Result};

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let symbol = args.next().context("expected symbol")?;
    let path = args.next().context("expected document path")?;
    let source = args.next().context("expected official document URL")?;
    println!(
        "{}",
        serde_json::to_string_pretty(&reit_metrics::extract_file(
            &symbol,
            std::path::Path::new(&path),
            &source,
        )?)?
    );
    Ok(())
}
