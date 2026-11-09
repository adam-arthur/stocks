use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use anyhow::{Context, Result, ensure};
use reqwest::{Client, Url};
use scraper::{Html, Selector};

use crate::companies::Company;

pub fn validate_url(company: &Company, source: &str) -> Result<Url> {
    let url = Url::parse(source)?;
    ensure!(
        url.scheme() == "https" && url.username().is_empty() && url.password().is_none(),
        "expected an official HTTPS source"
    );
    ensure!(
        company.hosts.contains(&url.host_str().unwrap_or_default())
            || (company.official_url)(&url)
            || (url.host_str() == Some("www.sec.gov")
                && url
                    .path()
                    .starts_with(&format!("/Archives/edgar/data/{}/", company.cik))),
        "source is not a researched official host or this company's EDGAR archive"
    );
    Ok(url)
}

pub struct Sources {
    pub(crate) sec: sec_client::SecClient,
    http: Client,
    cache: PathBuf,
}

impl Sources {
    pub fn new(cache: &Path) -> Result<Self> {
        Ok(Self {
            sec: sec_client::SecClient::from_env()?,
            http: Client::builder()
                .user_agent(format!(
                    "stocks-reit-metrics {}",
                    std::env::var("SEC_CONTACT_EMAIL").context("SEC_CONTACT_EMAIL must be set")?
                ))
                .timeout(Duration::from_secs(90))
                .redirect(reqwest::redirect::Policy::none())
                .build()?,
            cache: cache.to_owned(),
        })
    }

    pub async fn get(&self, company: &Company, source: &str) -> Result<String> {
        Ok(std::fs::read_to_string(
            self.download(company, source).await?,
        )?)
    }

    pub async fn download(&self, company: &Company, source: &str) -> Result<PathBuf> {
        self.download_inner(company, source, false).await
    }

    pub(crate) async fn download_inner(
        &self,
        company: &Company,
        source: &str,
        refresh: bool,
    ) -> Result<PathBuf> {
        let url = validate_url(company, source)?;
        let directory = self
            .cache
            .join(company.symbol)
            .join(url.host_str().unwrap());
        ensure!(
            url.path().split('/').all(|segment| segment != ".."),
            "invalid source path"
        );
        let path = directory.join(url.path().trim_start_matches('/'));
        ensure!(
            !url.path().ends_with('/') && url.query().is_none(),
            "source must identify a document without a query string"
        );
        if path.exists() && !refresh {
            return Ok(path);
        }
        let response = if matches!(
            url.host_str(),
            Some("www.sec.gov" | "data.sec.gov" | "sec.gov")
        ) {
            self.sec.get(url.as_str()).await?
        } else {
            tokio::time::sleep(Duration::from_millis(150)).await;
            self.http
                .get(url.clone())
                .send()
                .await?
                .error_for_status()?
        };
        ensure!(
            response.status().is_success() && response.url() == &url,
            "source redirected; use its canonical official URL"
        );
        let content = response.bytes().await?;
        ensure!(
            content.starts_with(b"%PDF-")
                || String::from_utf8_lossy(&content)
                    .to_lowercase()
                    .contains("<html"),
            "unsupported document format"
        );
        std::fs::create_dir_all(path.parent().unwrap())?;
        std::fs::write(&path, &content)?;
        Ok(path)
    }

    pub(crate) async fn additional_filings(
        &self,
        company: &Company,
        since: chrono::NaiveDate,
    ) -> Result<Vec<sec_client::EarningsFiling>> {
        let submissions: serde_json::Value = self
            .sec
            .get(&format!(
                "https://data.sec.gov/submissions/CIK{:010}.json",
                company.cik
            ))
            .await?
            .json()
            .await?;
        let mut groups = vec![submissions["filings"]["recent"].clone()];
        for file in submissions["filings"]["files"]
            .as_array()
            .into_iter()
            .flatten()
        {
            if file["filingTo"]
                .as_str()
                .is_some_and(|date| date >= since.to_string().as_str())
            {
                let name = file["name"]
                    .as_str()
                    .context("missing SEC submission filename")?;
                ensure!(
                    name.starts_with(&format!("CIK{:010}-submissions-", company.cik))
                        && name.ends_with(".json")
                        && !name.contains('/'),
                    "invalid SEC submission filename"
                );
                groups.push(
                    self.sec
                        .get(&format!("https://data.sec.gov/submissions/{name}"))
                        .await?
                        .json()
                        .await?,
                );
            }
        }
        let mut filings = Vec::new();
        for group in groups {
            for (index, form) in group["form"]
                .as_array()
                .context("missing SEC filing forms")?
                .iter()
                .enumerate()
            {
                if form != "8-K"
                    || !company.additional_items.is_some_and(|accept| {
                        accept(group["items"][index].as_str().unwrap_or_default())
                    })
                {
                    continue;
                }
                let filed_on = group["filingDate"][index]
                    .as_str()
                    .context("missing SEC filing date")?
                    .parse()?;
                if filed_on >= since {
                    filings.push(sec_client::EarningsFiling {
                        accession: group["accessionNumber"][index]
                            .as_str()
                            .context("missing SEC accession")?
                            .to_owned(),
                        filed_on,
                        form: "8-K".into(),
                    });
                }
            }
        }
        Ok(filings)
    }

    pub async fn filing_urls(
        &self,
        company: &Company,
        accession: &str,
        form: &str,
    ) -> Result<Vec<String>> {
        self.document_urls(
            company,
            accession,
            if form.starts_with("8-K") {
                "EX-99"
            } else {
                form
            },
        )
        .await
    }

    pub async fn publication_date(
        &self,
        company: &Company,
        filing: &sec_client::EarningsFiling,
    ) -> Result<chrono::NaiveDate> {
        ensure!(
            filing.form.starts_with("8-K"),
            "announcement date requires an 8-K with Item 2.02"
        );
        let urls = self
            .document_urls(company, &filing.accession, &filing.form)
            .await?;
        ensure!(
            urls.len() == 1,
            "expected one primary 8-K for {}",
            filing.accession
        );
        crate::publication::extract(&self.get(company, &urls[0]).await?, filing.filed_on)
            .with_context(|| format!("{}: {}", company.symbol, filing.accession))
    }

    async fn document_urls(
        &self,
        company: &Company,
        accession: &str,
        kind: &str,
    ) -> Result<Vec<String>> {
        ensure!(
            accession.len() == 20
                && accession
                    .bytes()
                    .enumerate()
                    .all(|(i, b)| if i == 10 || i == 13 {
                        b == b'-'
                    } else {
                        b.is_ascii_digit()
                    }),
            "invalid accession"
        );
        let base = format!(
            "https://www.sec.gov/Archives/edgar/data/{}/{}/",
            company.cik,
            accession.replace('-', "")
        );
        let document = self
            .get(company, &format!("{base}{accession}-index.html"))
            .await?;
        let html = Html::parse_document(&document);
        let cells = Selector::parse("td").unwrap();
        let link = Selector::parse("a[href]").unwrap();
        let mut result = Vec::new();
        for row in html.select(&Selector::parse("table.tableFile tr").unwrap()) {
            let cells: Vec<_> = row.select(&cells).collect();
            if cells.len() < 4 || !cells[3].text().collect::<String>().trim().starts_with(kind) {
                continue;
            }
            if let Some(href) = cells[2]
                .select(&link)
                .next()
                .and_then(|link| link.value().attr("href"))
            {
                let href = href.split_once("?doc=").map_or(href, |(_, path)| path);
                let url = Url::parse(&base)?.join(href)?;
                validate_url(company, url.as_str())?;
                result.push(url.to_string());
            }
        }
        Ok(result)
    }
}
