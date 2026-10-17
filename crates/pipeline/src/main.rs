use anyhow::{Result, ensure};

mod logging;

#[tokio::main]
async fn main() -> Result<()> {
    ensure!(std::env::args().len() == 1, "the scan takes no arguments");
    match dotenvy::dotenv() {
        Ok(_) => {}
        Err(dotenvy::Error::Io(error)) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    let progress = logging::init();
    let store = ingest::Store::open("data")?;
    ingest::update(&store, &progress).await
}
