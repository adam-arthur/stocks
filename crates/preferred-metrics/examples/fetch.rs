use anyhow::{Context, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let mut arguments = std::env::args().skip(1);
    let symbol = arguments
        .next()
        .context("usage: fetch SYMBOL [HTML_FILE]")?;
    let preferred = if let Some(path) = arguments.next() {
        preferred_metrics::parse(&symbol, &String::from_utf8_lossy(&std::fs::read(path)?))?
    } else {
        preferred_metrics::PreferredClient::new()?
            .fetch(&symbol)
            .await?
    };
    println!("{}", serde_json::to_string_pretty(&preferred)?);
    Ok(())
}
