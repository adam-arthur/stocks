use std::{
    collections::BTreeMap,
    fs::{self, File},
    io::{ErrorKind, Write},
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, ensure};
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Serialize, de::DeserializeOwned};

use crate::models::{CorporateAction, Failure, FinancialsSync, Metadata, ReitQuarter};

pub struct Store {
    root: PathBuf,
    _lock: File,
}

impl Store {
    pub fn open(root: impl Into<PathBuf>) -> Result<Self> {
        let root = root.into();
        fs::create_dir_all(root.join("stocks"))?;
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

    pub fn reit(&self, symbol: &str) -> Result<Vec<ReitQuarter>> {
        Ok(read_optional(&self.symbol_path(symbol)?.join("reit.json"))?.unwrap_or_default())
    }

    pub fn debt(&self, symbol: &str) -> Result<Option<reit_metrics::debt::Debt>> {
        read_optional(&self.symbol_path(symbol)?.join("debt.json"))
    }

    pub fn debt_failures(&self) -> Result<Vec<Failure>> {
        Ok(read_optional(&self.root.join("failed_reit_debt.json"))?.unwrap_or_default())
    }

    pub fn save_debt(&self, symbol: &str, debt: &reit_metrics::debt::Debt) -> Result<()> {
        ensure!(
            self.debt(symbol)?
                .is_none_or(|previous| previous.as_of <= debt.as_of),
            "refusing to replace a newer debt snapshot for {symbol}"
        );
        write_atomic(&self.symbol_path(symbol)?.join("debt.json"), debt)
    }

    pub fn save_debt_failures(&self, failures: &[Failure]) -> Result<()> {
        write_atomic(&self.root.join("failed_reit_debt.json"), failures)
    }

    pub fn preferred(&self, symbol: &str) -> Result<Option<preferred_metrics::Preferred>> {
        read_optional(&self.symbol_path(symbol)?.join("preferred.json"))
    }

    pub fn preferred_failures(&self) -> Result<Vec<Failure>> {
        Ok(read_optional(&self.root.join("failed_preferred.json"))?.unwrap_or_default())
    }

    pub fn save_preferred_failures(&self, failures: &[Failure]) -> Result<()> {
        write_atomic(&self.root.join("failed_preferred.json"), failures)
    }

    pub fn save_preferred(
        &self,
        symbol: &str,
        preferred: &preferred_metrics::Preferred,
    ) -> Result<()> {
        write_atomic(&self.symbol_path(symbol)?.join("preferred.json"), preferred)
    }

    pub fn save_reit(&self, symbol: &str, quarters: &[ReitQuarter]) -> Result<()> {
        write_atomic(&self.symbol_path(symbol)?.join("reit.json"), quarters)
    }

    pub fn prices(&self, symbol: &str) -> Result<BTreeMap<NaiveDate, f64>> {
        Ok(read_optional(&self.symbol_path(symbol)?.join("prices.json"))?.unwrap_or_default())
    }

    pub fn prices_sync(&self) -> Result<Option<DateTime<Utc>>> {
        read_optional(&self.root.join("prices_sync.json"))
    }

    pub fn save_prices_sync(&self, updated_at: DateTime<Utc>) -> Result<()> {
        write_atomic(&self.root.join("prices_sync.json"), &updated_at)
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

    pub fn sec_directory(&self) -> Result<PathBuf> {
        let path = self.root.join("sec");
        fs::create_dir_all(&path)?;
        Ok(path)
    }

    pub fn financials_sync(&self) -> Result<Option<FinancialsSync>> {
        read_optional(&self.root.join("sec/sync.json"))
    }

    pub fn save_financials_sync(&self, sync: &FinancialsSync) -> Result<()> {
        write_atomic(&self.root.join("sec/sync.json"), sync)
    }

    pub fn has_company_facts(&self, cik: u64) -> bool {
        self.company_facts_path(cik).is_file()
    }

    pub fn company_facts(&self, cik: u64) -> Result<Option<serde_json::Value>> {
        read_optional(&self.company_facts_path(cik))
    }

    pub fn save_company_facts(&self, cik: u64, facts: &serde_json::Value) -> Result<()> {
        ensure!(
            facts.get("cik").is_none() || sec_client::company_cik(facts) == Some(cik),
            "company CIK mismatch"
        );
        write_json_atomic(&self.company_facts_path(cik), facts, false)
    }

    fn company_facts_path(&self, cik: u64) -> PathBuf {
        self.root
            .join(format!("sec/company_facts/CIK{cik:010}.json"))
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
        Ok(self.root.join("stocks").join(symbol))
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
    write_json_atomic(path, value, true)
}

fn write_json_atomic<T: Serialize + ?Sized>(path: &Path, value: &T, pretty: bool) -> Result<()> {
    fs::create_dir_all(path.parent().context("missing parent directory")?)?;
    let mut file = tempfile::NamedTempFile::new_in(path.parent().unwrap())?;
    {
        let mut writer = std::io::BufWriter::new(&mut file);
        if pretty {
            serde_json::to_writer_pretty(&mut writer, value)?;
        } else {
            serde_json::to_writer(&mut writer, value)?;
        }
        writer.flush()?;
    }
    file.write_all(b"\n")?;
    file.as_file().sync_all()?;
    file.persist(path)?;
    Ok(())
}
