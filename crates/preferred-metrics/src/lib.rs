use std::{collections::HashMap, time::Duration};

use anyhow::{Context, Result, ensure};
use chrono::NaiveDate;
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::time::{Instant, sleep_until};

const REQUEST_INTERVAL: Duration = Duration::from_millis(600);

#[derive(Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum RateType {
    Fixed,
    Floating,
    FixedToFloating,
    Reset,
    Other,
    Unknown,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DividendTerms {
    pub rate_type: RateType,
    pub fixed_annual_coupon: Option<f64>,
    pub first_reset_date: Option<NaiveDate>,
    pub reset_interval_months: Option<u32>,
    pub original_benchmark: Option<String>,
    pub coupon_floor: Option<f64>,
    pub coupon_cap: Option<f64>,
    pub benchmark_floor: Option<f64>,
    pub benchmark_cap: Option<f64>,
    pub spread_bps: Option<f64>,
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum CallSchedule {
    Anytime,
    ResetDatesOnly,
    Other,
    Noncallable,
    Unknown,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Prospectus {
    pub prospectus_url: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Preferred {
    pub sec: Option<Prospectus>,
    pub series: Option<String>,
    pub redeemable: Option<bool>,
    pub call_date: Option<NaiveDate>,
    pub call_price: Option<f64>,
    pub call_schedule: Option<CallSchedule>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub change_of_control_conversion_threshold_price: Option<f64>,
    pub change_of_control_redemption_price: Option<f64>,
    pub perpetual: Option<bool>,
    pub maturity_date: Option<NaiveDate>,
    pub cumulative: Option<bool>,
    pub convertible: Option<bool>,
    pub conversion_ratio: Option<f64>,
    pub conversion_price: Option<f64>,
    pub liquidation_preference: f64,
    pub original_coupon: Option<f64>,
    pub dividend_terms: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub structured_dividend_terms: Option<DividendTerms>,
    pub pay_period: Option<String>,
    pub pay_dates: Option<String>,
}

impl Preferred {
    pub fn has_filing_terms(&self) -> bool {
        self.sec
            .as_ref()
            .is_some_and(|sec| sec.prospectus_url.is_some())
            && self.structured_dividend_terms.is_some()
            && self.call_schedule.is_some()
    }
}

pub struct PreferredClient {
    http: Client,
    next_request: Instant,
    stopped: bool,
}

impl PreferredClient {
    pub fn new() -> Result<Self> {
        Ok(Self {
            http: Client::builder()
                .user_agent("stocks-preferred-metrics")
                .connect_timeout(Duration::from_secs(15))
                .timeout(Duration::from_secs(45))
                .redirect(reqwest::redirect::Policy::none())
                .build()?,
            next_request: Instant::now(),
            stopped: false,
        })
    }

    pub fn is_stopped(&self) -> bool {
        self.stopped
    }

    async fn request(&mut self, path: &str) -> Result<Value> {
        ensure!(
            !self.stopped,
            "preferred metadata requests stopped for this run"
        );
        let mut retried = false;
        loop {
            sleep_until(self.next_request).await;
            self.next_request = Instant::now() + REQUEST_INTERVAL;
            let response = self
                .http
                .get(format!("https://preferredstock.ai/api/v1/{path}"))
                .send()
                .await
                .inspect_err(|_| self.stopped = true)?;
            let status = response.status();
            if status == StatusCode::TOO_MANY_REQUESTS && !retried {
                let delay = response
                    .headers()
                    .get(reqwest::header::RETRY_AFTER)
                    .map_or(Some(60), |value| value.to_str().ok()?.parse::<u64>().ok());
                if let Some(seconds @ 0..=60) = delay {
                    self.next_request =
                        Instant::now() + REQUEST_INTERVAL.max(Duration::from_secs(seconds));
                    retried = true;
                    continue;
                }
            }
            if status == StatusCode::FORBIDDEN
                || status == StatusCode::TOO_MANY_REQUESTS
                || status.is_server_error()
            {
                self.stopped = true;
            }
            ensure!(status.is_success(), "Preferred Stock AI returned {status}");
            return Ok(response.json().await.inspect_err(|_| self.stopped = true)?);
        }
    }

    pub async fn fetch(&mut self, symbol: &str) -> Result<Preferred> {
        let symbol = normalize_symbol(symbol)?;
        let response = self.request(&format!("symbol/{symbol}")).await?;
        parse_security(&symbol, &response["security"])
    }

    pub async fn fetch_issuer(
        &mut self,
        issuer: &str,
        symbols: &[&str],
    ) -> Result<HashMap<String, Preferred>> {
        ensure!(
            !issuer.is_empty()
                && issuer
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || b".-".contains(&c)),
            "invalid issuer"
        );
        let response = self.request(&format!("issuer/{issuer}")).await?;
        let securities = response["securities"]
            .as_array()
            .context("missing issuer securities")?;
        let mut matched = HashMap::new();
        for &symbol in symbols {
            let normalized = normalize_symbol(symbol)?;
            if let Some(row) = securities
                .iter()
                .find(|row| matches_symbol(row, &normalized))
                && row["sec"]["prospectus_url"].is_string()
                && let Ok(preferred) = parse_security(&normalized, row)
            {
                matched.insert(symbol.to_owned(), preferred);
            }
        }
        Ok(matched)
    }
}

pub fn parse(symbol: &str, json: &str) -> Result<Preferred> {
    let response: Value = serde_json::from_str(json)?;
    parse_security(&normalize_symbol(symbol)?, &response["security"])
}

#[derive(Deserialize)]
struct Security {
    sec: Option<Prospectus>,
    #[serde(rename = "type")]
    kind: String,
    series: Option<String>,
    call: Option<Call>,
    perpetual: Option<bool>,
    maturity_date: Option<NaiveDate>,
    cumulative: Option<bool>,
    convertible: Option<bool>,
    par: Option<f64>,
    coupon_pct: Option<f64>,
    pay_frequency: Option<String>,
}

#[derive(Deserialize)]
struct Call {
    date: Option<NaiveDate>,
}

fn matches_symbol(row: &Value, symbol: &str) -> bool {
    row["symbol"].as_str() == Some(symbol)
        || row["aliases"].as_array().is_some_and(|aliases| {
            aliases.iter().any(|alias| {
                alias
                    .as_str()
                    .is_some_and(|alias| alias.eq_ignore_ascii_case(symbol))
            })
        })
}

fn parse_security(symbol: &str, row: &Value) -> Result<Preferred> {
    ensure!(
        matches_symbol(row, symbol),
        "API security does not match {symbol}"
    );
    let security = Security::deserialize(row)?;
    ensure!(
        security.kind == "preferred",
        "{symbol} is classified as {}",
        security.kind
    );
    let par = security.par.context("missing liquidation preference")?;
    ensure!(par > 0.0, "invalid liquidation preference");
    ensure!(
        security.perpetual != Some(true) || security.maturity_date.is_none(),
        "perpetual preferred has a maturity date"
    );
    ensure!(
        security.coupon_pct.is_none_or(|rate| rate >= 0.0),
        "invalid coupon"
    );
    let original_coupon = security.coupon_pct.map(|rate| rate / 100.0);
    let call_date = security.call.and_then(|call| call.date);
    Ok(Preferred {
        sec: Some(security.sec.unwrap_or(Prospectus {
            prospectus_url: None,
        })),
        series: security.series,
        redeemable: call_date.map(|_| true),
        call_date,
        call_price: None,
        call_schedule: None,
        change_of_control_conversion_threshold_price: None,
        change_of_control_redemption_price: None,
        perpetual: security.perpetual,
        maturity_date: security.maturity_date,
        cumulative: security.cumulative,
        convertible: security.convertible,
        conversion_ratio: None,
        conversion_price: None,
        liquidation_preference: par,
        original_coupon,
        dividend_terms: None,
        structured_dividend_terms: None,
        pay_period: security.pay_frequency,
        pay_dates: None,
    })
}

fn normalize_symbol(symbol: &str) -> Result<String> {
    let symbol = symbol.to_ascii_uppercase();
    ensure!(
        !symbol.is_empty()
            && symbol
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || b".-^".contains(&c)),
        "invalid preferred symbol: {symbol}"
    );
    Ok(if symbol.contains("-P-") {
        symbol
    } else if let Some((issuer, series)) = symbol.split_once(".PR") {
        format!("{issuer}-P-{series}")
    } else if let Some((issuer, series)) = symbol.split_once('-') {
        format!(
            "{issuer}-P-{}",
            series
                .strip_prefix('P')
                .filter(|series| !series.is_empty())
                .unwrap_or(series)
        )
    } else if let Some((issuer, series)) = symbol.split_once('^') {
        format!("{issuer}-P-{series}")
    } else {
        symbol
    })
}
