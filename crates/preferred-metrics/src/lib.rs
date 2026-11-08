use std::{collections::BTreeMap, time::Duration};

use anyhow::{Context, Result, bail, ensure};
use chrono::NaiveDate;
use reqwest::{Client, StatusCode};
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use tokio::time::{Instant, sleep_until};

const REQUEST_INTERVAL: Duration = Duration::from_millis(200);

#[derive(Debug, Deserialize, Serialize)]
pub struct Preferred {
    pub series: Option<String>,
    pub redeemable: Option<bool>,
    pub call_date: Option<NaiveDate>,
    pub call_price: Option<f64>,
    pub perpetual: Option<bool>,
    pub maturity_date: Option<NaiveDate>,
    pub cumulative: Option<bool>,
    pub convertible: Option<bool>,
    pub conversion_ratio: Option<f64>,
    pub conversion_price: Option<f64>,
    pub liquidation_preference: f64,
    pub original_coupon: Option<f64>,
    pub dividend_terms: Option<String>,
    pub pay_period: Option<String>,
    pub pay_dates: Option<String>,
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

    pub async fn fetch(&mut self, symbol: &str) -> Result<Preferred> {
        ensure!(
            !self.stopped,
            "preferred metadata requests stopped for this run"
        );
        let symbol = normalize_symbol(symbol)?;
        sleep_until(self.next_request).await;
        self.next_request = Instant::now() + REQUEST_INTERVAL;
        let result = self
            .http
            .get(format!("https://preferredstock.ai/symbol/{}", symbol))
            .send()
            .await;
        self.next_request = Instant::now() + REQUEST_INTERVAL;
        let response = match result {
            Ok(response) => response,
            Err(error) => {
                self.stopped = true;
                return Err(error.into());
            }
        };
        if matches!(
            response.status(),
            StatusCode::FORBIDDEN | StatusCode::TOO_MANY_REQUESTS
        ) || response.status().is_server_error()
        {
            self.stopped = true;
            bail!(
                "Preferred Stock AI returned {}; stopping requests",
                response.status()
            );
        }
        ensure!(
            response.status().is_success(),
            "Preferred Stock AI returned {}",
            response.status()
        );
        let result = response.text().await;
        self.next_request = Instant::now() + REQUEST_INTERVAL;
        let html = match result {
            Ok(html) => html,
            Err(error) => {
                self.stopped = true;
                return Err(error.into());
            }
        };
        parse(&symbol, &html)
    }
}

pub fn parse(symbol: &str, html: &str) -> Result<Preferred> {
    let symbol = normalize_symbol(symbol)?;
    let document = Html::parse_document(html);
    ensure!(
        document
            .select(&Selector::parse("link[rel=canonical]").unwrap())
            .any(|link| link.value().attr("href")
                == Some(format!("https://preferredstock.ai/symbol/{symbol}").as_str())),
        "preferred page does not identify {symbol}"
    );
    let label_selector = Selector::parse(".k").unwrap();
    let value_selector = Selector::parse(".v").unwrap();
    let fields = document
        .select(&Selector::parse(".stat").unwrap())
        .filter_map(|stat| {
            Some((
                stat.select(&label_selector)
                    .next()?
                    .text()
                    .collect::<String>()
                    .trim()
                    .to_owned(),
                stat.select(&value_selector)
                    .next()?
                    .text()
                    .collect::<Vec<_>>()
                    .join(" ")
                    .split_whitespace()
                    .collect::<Vec<_>>()
                    .join(" "),
            ))
        })
        .collect::<BTreeMap<_, _>>();
    ensure!(
        fields.contains_key("Pay Frequency"),
        "missing preferred key stats"
    );
    let text = |key: &str| {
        fields
            .get(key)
            .filter(|value| {
                !matches!(
                    value.to_ascii_lowercase().as_str(),
                    "" | "n/a" | "n.a." | "na" | "none" | "unknown" | "-" | "--" | "—"
                )
            })
            .map(String::as_str)
    };
    let number = |key: &str| -> Result<Option<f64>> {
        text(key)
            .map(|value| {
                if matches!(key, "Conversion Ratio" | "Conversion Price")
                    && let Some((start, end)) =
                        value.split_once('–').or_else(|| value.split_once(" - "))
                {
                    let start = start
                        .trim()
                        .trim_start_matches('$')
                        .parse::<f64>()
                        .with_context(|| format!("invalid {key}: {value}"))?;
                    let end = end
                        .trim()
                        .trim_start_matches('$')
                        .parse::<f64>()
                        .with_context(|| format!("invalid {key}: {value}"))?;
                    ensure!(
                        start.is_finite() && end.is_finite() && start >= 0.0 && end >= 0.0,
                        "invalid {key}: {value}"
                    );
                    return Ok(start / 2.0 + end / 2.0);
                }
                let number = value
                    .trim_start_matches('$')
                    .trim_end_matches('%')
                    .replace(',', "")
                    .parse::<f64>()
                    .with_context(|| format!("invalid {key}: {value}"))?;
                ensure!(
                    number.is_finite() && number >= 0.0,
                    "invalid {key}: {value}"
                );
                Ok(number)
            })
            .transpose()
    };
    let boolean = |key: &str| -> Result<Option<bool>> {
        text(key)
            .map(|value| match value.split_whitespace().next().unwrap() {
                "Yes" => Ok(true),
                "No" => Ok(false),
                _ => bail!("invalid {key}: {value}"),
            })
            .transpose()
    };
    let date = |key: &str| -> Result<Option<NaiveDate>> {
        text(key)
            .filter(|value| !(key == "Call Date" && *value == "redeemable (date n/a)"))
            .map(|value| {
                NaiveDate::parse_from_str(value.split_whitespace().next().unwrap(), "%Y-%m-%d")
                    .with_context(|| format!("invalid {key}: {value}"))
            })
            .transpose()
    };
    let preferred = Preferred {
        series: text("Series").map(str::to_owned),
        redeemable: boolean("Redeemable")?,
        call_date: date("Call Date")?,
        call_price: number("Call Price")?,
        perpetual: text("Maturity").map(|value| value == "Perpetual"),
        maturity_date: if text("Maturity") == Some("Perpetual") {
            None
        } else {
            date("Maturity")?
        },
        cumulative: boolean("Cumulative")?,
        convertible: boolean("Convertible")?,
        conversion_ratio: number("Conversion Ratio")?,
        conversion_price: number("Conversion Price")?,
        liquidation_preference: number("Liq. Preference")?
            .context("missing liquidation preference")?,
        original_coupon: text("Original Coupon")
            .or_else(|| text("Initial Rate"))
            .and_then(|value| value.split_once('%'))
            .and_then(|(rate, _)| rate.trim().parse::<f64>().ok())
            .filter(|rate| rate.is_finite() && *rate >= 0.0)
            .map(|rate| rate / 100.0),
        dividend_terms: {
            let terms = [
                text("Original Coupon").or_else(|| text("Initial Rate")),
                text("Floating Rate"),
            ]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();
            (!terms.is_empty()).then(|| terms.join("; "))
        },
        pay_period: text("Pay Frequency").map(str::to_owned),
        pay_dates: text("Pay Dates").map(str::to_owned),
    };
    ensure!(
        preferred.liquidation_preference > 0.0,
        "invalid liquidation preference"
    );
    ensure!(
        !(preferred.perpetual == Some(true) && preferred.maturity_date.is_some()),
        "perpetual preferred has a maturity date"
    );
    Ok(preferred)
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
