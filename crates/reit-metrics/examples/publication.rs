use anyhow::{Context, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let symbol = args.next().context("expected symbol")?;
    let accession = args.next().context("expected accession")?;
    let filed_on = args.next().context("expected filing date")?.parse()?;
    let cache = args.next().context("expected cache directory")?;
    println!(
        "{}",
        reit_metrics::source::Sources::new(std::path::Path::new(&cache))?
            .publication_date(
                &reit_metrics::companies::company(&symbol)?,
                &sec_client::EarningsFiling {
                    accession,
                    filed_on,
                    form: "8-K".into(),
                },
            )
            .await?
    );
    Ok(())
}
