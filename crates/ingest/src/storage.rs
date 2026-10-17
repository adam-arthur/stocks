use std::{
    collections::BTreeMap,
    fs::{self, File},
    io::{ErrorKind, Write},
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, ensure};
use chrono::NaiveDate;
use serde::{Serialize, de::DeserializeOwned};

use crate::models::{CorporateAction, Failure, Metadata};

pub struct Store {
    root: PathBuf,
    _lock: File,
}

impl Store {
    pub fn open(root: impl Into<PathBuf>) -> Result<Self> {
        let root = root.into();
        fs::create_dir_all(&root)?;
        let lock = File::options()
            .create(true)
            .truncate(false)
            .write(true)
            .open(root.join(".lock"))?;
        lock.try_lock()
            .context("another pipeline is using this data directory")?;
        Ok(Self { root, _lock: lock })
    }

    pub fn metadata(&self, symbol: &str) -> Result<Option<Metadata>> {
        read_optional(&self.symbol_path(symbol)?.join("meta.json"))
    }

    pub fn failures(&self) -> Result<Vec<Failure>> {
        Ok(read_optional(&self.root.join("failed_categorization.json"))?.unwrap_or_default())
    }

    pub fn pending_research(&self) -> Result<Vec<Failure>> {
        Ok(read_optional(&self.root.join("pending_research.json"))?.unwrap_or_default())
    }

    pub fn save_pending_research(&self, pending: &[Failure]) -> Result<()> {
        write_atomic(&self.root.join("pending_research.json"), pending)
    }

    pub fn save_failures(&self, failures: &[Failure]) -> Result<()> {
        write_atomic(&self.root.join("failed_categorization.json"), failures)
    }

    pub fn save_metadata(&self, metadata: &Metadata) -> Result<()> {
        write_atomic(
            &self.symbol_path(&metadata.symbol)?.join("meta.json"),
            metadata,
        )
    }

    pub fn prices(&self, symbol: &str) -> Result<BTreeMap<NaiveDate, f64>> {
        Ok(read_optional(&self.symbol_path(symbol)?.join("prices.json"))?.unwrap_or_default())
    }

    pub fn save_prices(&self, symbol: &str, prices: &BTreeMap<NaiveDate, f64>) -> Result<()> {
        ensure!(self.metadata(symbol)?.is_some(), "no metadata for {symbol}");
        write_atomic(&self.symbol_path(symbol)?.join("prices.json"), prices)
    }

    pub fn corporate_actions(&self, symbol: &str) -> Result<Vec<CorporateAction>> {
        Ok(
            read_optional(&self.symbol_path(symbol)?.join("corporate_actions.json"))?
                .unwrap_or_default(),
        )
    }

    pub fn save_corporate_actions(&self, symbol: &str, actions: &[CorporateAction]) -> Result<()> {
        ensure!(self.metadata(symbol)?.is_some(), "no metadata for {symbol}");
        write_atomic(
            &self.symbol_path(symbol)?.join("corporate_actions.json"),
            actions,
        )
    }

    pub fn corporate_actions_sync(&self) -> Result<BTreeMap<String, NaiveDate>> {
        Ok(read_optional(&self.root.join("corporate_actions_sync.json"))?.unwrap_or_default())
    }

    pub fn save_corporate_actions_sync(&self, dates: &BTreeMap<String, NaiveDate>) -> Result<()> {
        write_atomic(&self.root.join("corporate_actions_sync.json"), dates)
    }

    fn symbol_path(&self, symbol: &str) -> Result<PathBuf> {
        ensure!(
            !symbol.is_empty()
                && symbol != "."
                && symbol != ".."
                && symbol
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || b".-_^".contains(&c)),
            "unsafe symbol: {symbol}"
        );
        Ok(self.root.join(symbol))
    }
}

fn read_optional<T: DeserializeOwned>(path: &Path) -> Result<Option<T>> {
    match fs::read(path) {
        Ok(bytes) => {
            Ok(Some(serde_json::from_slice(&bytes).with_context(|| {
                format!("invalid JSON in {}", path.display())
            })?))
        }
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

fn write_atomic<T: Serialize + ?Sized>(path: &Path, value: &T) -> Result<()> {
    let bytes = serde_json::to_vec_pretty(value)?;
    fs::create_dir_all(path.parent().context("missing parent directory")?)?;
    let mut file = tempfile::NamedTempFile::new_in(path.parent().unwrap())?;
    file.write_all(&bytes)?;
    file.write_all(b"\n")?;
    file.as_file().sync_all()?;
    file.persist(path)?;
    Ok(())
}
