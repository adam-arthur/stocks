use anyhow::{Context, Result};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    fs::{self, File},
    io::ErrorKind,
    path::{Path, PathBuf},
};
use tempfile::NamedTempFile;

#[derive(Serialize, Deserialize)]
pub struct Alert {
    pub symbol: String,
    pub rule: String,
    pub observed_on: NaiveDate,
    pub value: f64,
    pub percentile: f64,
    pub std_devs_from_median: Option<f64>,
}

#[derive(Serialize, Deserialize)]
pub enum Notification {
    InitialSummary(Vec<Alert>),
    Entered(Alert),
}

#[derive(Default, Serialize, Deserialize)]
pub struct State {
    pub active: BTreeMap<String, BTreeSet<String>>,
    pub pending: VecDeque<Notification>,
}

pub struct Store {
    directory: PathBuf,
    _lock: File,
}

impl Store {
    pub fn open(directory: &Path) -> Result<Self> {
        fs::create_dir_all(directory)?;
        let lock = File::options()
            .create(true)
            .append(true)
            .open(directory.join("state.lock"))?;
        lock.lock()?;
        Ok(Self {
            directory: directory.into(),
            _lock: lock,
        })
    }

    pub fn load(&self) -> Result<State> {
        let path = self.directory.join("state.json");
        match fs::read(&path) {
            Ok(bytes) => Ok(serde_json::from_slice(&bytes)
                .with_context(|| format!("reading {}", path.display()))?),
            Err(e) if e.kind() == ErrorKind::NotFound => Ok(State::default()),
            Err(e) => Err(e.into()),
        }
    }

    pub fn save(&self, state: &State) -> Result<()> {
        let mut file = NamedTempFile::new_in(&self.directory)?;
        serde_json::to_writer(file.as_file_mut(), state)?;
        file.as_file().sync_all()?;
        file.persist(self.directory.join("state.json"))?;
        #[cfg(unix)]
        File::open(&self.directory)?.sync_all()?;
        Ok(())
    }
}
