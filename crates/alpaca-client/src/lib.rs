use std::{
    collections::BTreeMap,
    env,
    sync::{Arc, Mutex},
    time::Duration,
};

use chrono::{DateTime, NaiveDate, Utc};
use reqwest::{
    Client, StatusCode, Url,
    header::{HeaderMap, HeaderValue},
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("missing or invalid environment variable: {0}")]
    Environment(&'static str),
    #[error("invalid API URL")]
    InvalidUrl,
    #[error("invalid API credentials")]
    InvalidCredentials,
    #[error("symbol must not be empty")]
    EmptySymbol,
    #[error("Alpaca returned {status}: {message}")]
    Api { status: StatusCode, message: String },
    #[error(transparent)]
    Http(#[from] reqwest::Error),
}

#[derive(Clone)]
pub struct AlpacaClient {
    http: Client,
    trading_url: Url,
    data_url: Url,
    next_request: Arc<Mutex<tokio::time::Instant>>,
}

#[derive(Debug, Clone, Copy, Default, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Feed {
    #[default]
    Iex,
    Sip,
    DelayedSip,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Asset {
    pub id: String,
    pub symbol: String,
    pub name: String,
    pub exchange: String,
    pub status: String,
    pub tradable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatestTrade {
    pub symbol: String,
    pub trade: Trade,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trade {
    #[serde(rename = "p")]
    pub price: f64,
    #[serde(rename = "s")]
    pub size: f64,
    #[serde(rename = "t")]
    pub timestamp: DateTime<Utc>,
    #[serde(rename = "x")]
    pub exchange: String,
}

#[derive(Debug, Deserialize)]
pub struct BarsPage {
    #[serde(default)]
    pub bars: BTreeMap<String, Vec<Bar>>,
    pub next_page_token: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Bar {
    pub t: DateTime<Utc>,
    pub c: f64,
}

#[derive(Debug, Deserialize)]
pub struct CorporateActionsPage {
    pub corporate_actions: BTreeMap<String, Vec<serde_json::Value>>,
    pub next_page_token: Option<String>,
}

impl AlpacaClient {
    pub fn new(key: &str, secret: &str, trading_url: &str, data_url: &str) -> Result<Self, Error> {
        let mut headers = HeaderMap::new();
        for (name, value) in [("apca-api-key-id", key), ("apca-api-secret-key", secret)] {
            if value.trim().is_empty() {
                return Err(Error::InvalidCredentials);
            }
            let mut value = HeaderValue::from_str(value).map_err(|_| Error::InvalidCredentials)?;
            value.set_sensitive(true);
            headers.insert(name, value);
        }
        Ok(Self {
            http: Client::builder()
                .default_headers(headers)
                .timeout(Duration::from_secs(30))
                .redirect(reqwest::redirect::Policy::none())
                .build()?,
            trading_url: base_url(trading_url)?,
            data_url: base_url(data_url)?,
            next_request: Arc::new(Mutex::new(tokio::time::Instant::now())),
        })
    }

    pub fn from_env() -> Result<Self, Error> {
        Self::new(
            &env::var("ALPACA_KEY").map_err(|_| Error::Environment("ALPACA_KEY"))?,
            &env::var("ALPACA_SECRET").map_err(|_| Error::Environment("ALPACA_SECRET"))?,
            &env::var("ALPACA_TRADING_URL")
                .map_err(|_| Error::Environment("ALPACA_TRADING_URL"))?,
            &env::var("ALPACA_DATA_URL").map_err(|_| Error::Environment("ALPACA_DATA_URL"))?,
        )
    }

    pub async fn list_tradable_stocks(&self) -> Result<Vec<Asset>, Error> {
        Ok(self
            .get::<Vec<Asset>>(
                self.http
                    .get(
                        self.trading_url
                            .join("assets")
                            .map_err(|_| Error::InvalidUrl)?,
                    )
                    .query(&[("status", "active"), ("asset_class", "us_equity")]),
            )
            .await?
            .into_iter()
            .filter(|asset| asset.tradable)
            .collect())
    }

    pub async fn latest_trade(&self, symbol: &str, feed: Feed) -> Result<LatestTrade, Error> {
        if symbol.trim().is_empty() {
            return Err(Error::EmptySymbol);
        }
        let mut url = self.data_url.clone();
        url.path_segments_mut()
            .map_err(|_| Error::InvalidUrl)?
            .pop_if_empty()
            .extend([
                "stocks",
                &symbol.trim().to_ascii_uppercase(),
                "trades",
                "latest",
            ]);
        self.get(self.http.get(url).query(&[("feed", feed)])).await
    }

    pub async fn daily_bars(
        &self,
        symbols: &[String],
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        page: Option<&str>,
    ) -> Result<BarsPage, Error> {
        self.get(
            self.http
                .get(
                    self.data_url
                        .join("stocks/bars")
                        .map_err(|_| Error::InvalidUrl)?,
                )
                .query(&[
                    ("symbols", symbols.join(",")),
                    ("timeframe", "1Day".into()),
                    ("feed", "sip".into()),
                    ("adjustment", "raw".into()),
                    ("sort", "asc".into()),
                    ("limit", "10000".into()),
                    ("start", start.to_rfc3339()),
                    ("end", end.to_rfc3339()),
                ])
                .query(
                    &page
                        .map(|value| ("page_token", value))
                        .into_iter()
                        .collect::<Vec<_>>(),
                ),
        )
        .await
    }

    pub async fn corporate_actions(
        &self,
        symbols: &[String],
        start: NaiveDate,
        end: NaiveDate,
        page: Option<&str>,
    ) -> Result<CorporateActionsPage, Error> {
        self.get(
            self.http
                .get(
                    self.data_url
                        .join("/v1/corporate-actions")
                        .map_err(|_| Error::InvalidUrl)?,
                )
                .query(&[
                    ("symbols", symbols.join(",")),
                    ("start", start.to_string()),
                    ("end", end.to_string()),
                    ("limit", "1000".into()),
                    ("sort", "asc".into()),
                ])
                .query(
                    &page
                        .map(|value| ("page_token", value))
                        .into_iter()
                        .collect::<Vec<_>>(),
                ),
        )
        .await
    }

    async fn get<T: DeserializeOwned>(&self, request: reqwest::RequestBuilder) -> Result<T, Error> {
        for attempt in 0..5 {
            let scheduled = {
                let mut next = self.next_request.lock().unwrap();
                let scheduled = (*next).max(tokio::time::Instant::now());
                *next = scheduled + Duration::from_millis(310);
                scheduled
            };
            tokio::time::sleep_until(scheduled).await;
            let response = match request
                .try_clone()
                .expect("GET request can be cloned")
                .send()
                .await
            {
                Ok(response) => response,
                Err(error) if attempt < 4 && (error.is_timeout() || error.is_connect()) => {
                    tokio::time::sleep(Duration::from_secs(1 << attempt)).await;
                    continue;
                }
                Err(error) => return Err(error.into()),
            };
            if attempt < 4
                && (response.status() == StatusCode::TOO_MANY_REQUESTS
                    || response.status().is_server_error())
            {
                let delay = response
                    .headers()
                    .get("retry-after")
                    .and_then(|value| value.to_str().ok())
                    .and_then(|value| value.parse::<u64>().ok())
                    .unwrap_or(if response.status() == StatusCode::TOO_MANY_REQUESTS {
                        60
                    } else {
                        1 << attempt
                    });
                let mut next = self.next_request.lock().unwrap();
                *next = (*next).max(tokio::time::Instant::now() + Duration::from_secs(delay));
                continue;
            }
            if !response.status().is_success() {
                return Err(Error::Api {
                    status: response.status(),
                    message: response.text().await?,
                });
            }
            return Ok(response.json().await?);
        }
        unreachable!()
    }
}

fn base_url(value: &str) -> Result<Url, Error> {
    let url =
        Url::parse(&format!("{}/", value.trim_end_matches('/'))).map_err(|_| Error::InvalidUrl)?;
    if !matches!(url.scheme(), "http" | "https")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(Error::InvalidUrl);
    }
    Ok(url)
}
