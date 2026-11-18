use std::{env, time::Duration};

use anyhow::{Context, Result, ensure};
use chrono::NaiveDate;
use reqwest::Client;
use serde::{Deserialize, Serialize};

pub const TREASURY_30_YEAR: &str = "DGS30";

#[derive(Clone)]
pub struct FredClient {
    http: Client,
    api_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Observation {
    pub date: NaiveDate,
    pub value: Option<f64>,
}

#[derive(Deserialize)]
struct Page {
    count: usize,
    observations: Vec<RawObservation>,
}

#[derive(Deserialize)]
struct RawObservation {
    date: NaiveDate,
    value: String,
}

impl FredClient {
    pub fn new(api_key: &str) -> Result<Self> {
        ensure!(!api_key.trim().is_empty(), "empty FRED API key");
        Ok(Self {
            http: Client::builder()
                .timeout(Duration::from_secs(30))
                .redirect(reqwest::redirect::Policy::none())
                .build()?,
            api_key: api_key.trim().to_owned(),
        })
    }

    pub fn from_env() -> Result<Self> {
        Self::new(&env::var("FRED_API_KEY").context("missing FRED_API_KEY")?)
    }

    pub async fn observations(
        &self,
        series_id: &str,
        start: NaiveDate,
        end: NaiveDate,
    ) -> Result<Vec<Observation>> {
        ensure!(!series_id.trim().is_empty(), "empty FRED series ID");
        ensure!(start <= end, "observation start is after end");
        let mut observations = Vec::new();
        loop {
            let response = self
                .http
                .get("https://api.stlouisfed.org/fred/series/observations")
                .query(&[
                    ("api_key", self.api_key.as_str()),
                    ("series_id", series_id.trim()),
                    ("file_type", "json"),
                ])
                .query(&[("observation_start", start), ("observation_end", end)])
                .query(&[("offset", observations.len())])
                .send()
                .await
                .map_err(reqwest::Error::without_url)?;
            ensure!(
                response.status().is_success(),
                "FRED returned {}",
                response.status()
            );
            let page: Page = response.json().await.map_err(reqwest::Error::without_url)?;
            ensure!(
                !page.observations.is_empty() || observations.len() >= page.count,
                "incomplete FRED observations"
            );
            for row in page.observations {
                observations.push(Observation {
                    date: row.date,
                    value: match row.value.as_str() {
                        "." => None,
                        value => Some(value.parse().with_context(|| {
                            format!("invalid {series_id} observation on {}", row.date)
                        })?),
                    },
                });
            }
            if observations.len() >= page.count {
                return Ok(observations);
            }
        }
    }
}
