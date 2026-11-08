use std::{
    collections::{BTreeMap, BTreeSet},
    env,
    fs::File,
    io::{BufReader, Read, Write},
    path::{Path, PathBuf},
    time::Duration,
};

use anyhow::{Context, Result, bail, ensure};
use chrono::NaiveDate;
use reqwest::{Client, Response, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tempfile::NamedTempFile;
use tokio::{io::AsyncWriteExt, sync::Mutex, time::Instant};
use zip::{ZipArchive, result::ZipError};

pub struct SecClient {
    http: Client,
    next_request: Mutex<Instant>,
}

#[derive(Debug, Deserialize)]
pub struct CompanyTicker {
    pub cik_str: u64,
    pub ticker: String,
    pub title: String,
}

pub fn cik_for_symbol(tickers: &[CompanyTicker], symbol: &str) -> Option<u64> {
    let candidates: BTreeSet<_> = tickers
        .iter()
        .filter(|company| company.ticker == symbol)
        .map(|company| company.cik_str)
        .collect();
    let candidates = if candidates.is_empty() {
        tickers
            .iter()
            .filter(|company| company.ticker.replace('.', "-") == symbol.replace('.', "-"))
            .map(|company| company.cik_str)
            .collect()
    } else {
        candidates
    };
    (candidates.len() == 1).then(|| *candidates.first().unwrap())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EarningsFiling {
    pub accession: String,
    pub filed_on: NaiveDate,
    pub form: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct FilingList {
    accession_number: Vec<String>,
    filing_date: Vec<NaiveDate>,
    form: Vec<String>,
    items: Vec<String>,
}

impl FilingList {
    fn earnings(self, since: NaiveDate) -> Result<Vec<EarningsFiling>> {
        ensure!(
            [self.filing_date.len(), self.form.len(), self.items.len()]
                .into_iter()
                .all(|length| length == self.accession_number.len()),
            "SEC filing columns have different lengths"
        );
        let mut filings = Vec::new();
        for (((accession, filed_on), form), items) in self
            .accession_number
            .into_iter()
            .zip(self.filing_date)
            .zip(self.form)
            .zip(self.items)
        {
            if filed_on >= since
                && (matches!(
                    form.as_str(),
                    "10-Q" | "10-Q/A" | "10-K" | "10-K/A" | "8-K/A"
                ) || (form == "8-K" && items.split(',').any(|item| item.trim() == "2.02")))
            {
                ensure!(
                    accession.len() == 20
                        && accession.bytes().enumerate().all(|(index, byte)| {
                            if index == 10 || index == 13 {
                                byte == b'-'
                            } else {
                                byte.is_ascii_digit()
                            }
                        }),
                    "invalid SEC accession number"
                );
                filings.push(EarningsFiling {
                    accession,
                    filed_on,
                    form,
                });
            }
        }
        Ok(filings)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FilingDocument {
    pub name: String,
    pub kind: String,
    pub text: String,
    pub images: Vec<PathBuf>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FilingDocuments {
    pub documents: Vec<FilingDocument>,
    pub unavailable: Vec<String>,
}

pub struct CompanyFactsArchive {
    archive: ZipArchive<BufReader<File>>,
    _file: NamedTempFile,
}

impl SecClient {
    pub fn new(email: &str) -> Result<Self> {
        ensure!(
            email.contains('@') && !email.chars().any(char::is_whitespace),
            "SEC_CONTACT_EMAIL must be a contact email address"
        );
        Ok(Self {
            http: Client::builder()
                .user_agent(format!("stocks/1.0 {email}"))
                .connect_timeout(Duration::from_secs(30))
                .read_timeout(Duration::from_secs(60))
                .build()?,
            next_request: Mutex::new(Instant::now()),
        })
    }

    pub fn from_env() -> Result<Self> {
        Self::new(&env::var("SEC_CONTACT_EMAIL").context("SEC_CONTACT_EMAIL must be set")?)
    }

    pub async fn company_tickers(&self) -> Result<Vec<CompanyTicker>> {
        let tickers = self
            .get("https://www.sec.gov/files/company_tickers.json")
            .await?
            .json::<BTreeMap<String, CompanyTicker>>()
            .await?;
        ensure!(!tickers.is_empty(), "SEC returned an empty ticker mapping");
        ensure!(
            tickers.values().all(|company| company.cik_str > 0
                && company.cik_str < 10_000_000_000
                && !company.ticker.is_empty()),
            "SEC returned an invalid ticker mapping"
        );
        Ok(tickers.into_values().collect())
    }

    pub async fn earnings_filings(
        &self,
        cik: u64,
        since: NaiveDate,
    ) -> Result<Vec<EarningsFiling>> {
        ensure!(cik > 0 && cik < 10_000_000_000, "invalid CIK");
        let submissions: Value = self
            .get(&format!(
                "https://data.sec.gov/submissions/CIK{cik:010}.json"
            ))
            .await?
            .json()
            .await?;
        ensure!(
            company_cik(&submissions) == Some(cik),
            "SEC submissions CIK mismatch"
        );
        let mut filings =
            serde_json::from_value::<FilingList>(submissions["filings"]["recent"].clone())?
                .earnings(since)?;
        for file in submissions["filings"]["files"]
            .as_array()
            .context("missing SEC historical filing list")?
        {
            if file["filingTo"]
                .as_str()
                .context("missing SEC history date")?
                .parse::<NaiveDate>()?
                < since
            {
                continue;
            }
            let name = file["name"]
                .as_str()
                .context("missing SEC history filename")?;
            ensure!(
                name.starts_with(&format!("CIK{cik:010}-submissions-"))
                    && name.ends_with(".json")
                    && name
                        .bytes()
                        .all(|byte| byte.is_ascii_alphanumeric() || b".-".contains(&byte)),
                "invalid SEC history filename"
            );
            filings.extend(
                self.get(&format!("https://data.sec.gov/submissions/{name}"))
                    .await?
                    .json::<FilingList>()
                    .await?
                    .earnings(since)?,
            );
        }
        filings.sort_by(|a, b| (&a.filed_on, &a.accession).cmp(&(&b.filed_on, &b.accession)));
        filings.dedup_by(|a, b| a.accession == b.accession);
        Ok(filings)
    }

    pub async fn filing_documents(
        &self,
        cik: u64,
        filing: &EarningsFiling,
        directory: &Path,
    ) -> Result<FilingDocuments> {
        ensure!(cik > 0 && cik < 10_000_000_000, "invalid CIK");
        ensure!(
            filing.accession.len() == 20
                && filing.accession.bytes().enumerate().all(|(index, byte)| {
                    if index == 10 || index == 13 {
                        byte == b'-'
                    } else {
                        byte.is_ascii_digit()
                    }
                }),
            "invalid SEC accession number"
        );
        let directory = directory.join(&filing.accession);
        std::fs::create_dir_all(&directory)?;
        let base = format!(
            "https://www.sec.gov/Archives/edgar/data/{cik}/{}/",
            filing.accession.replace('-', "")
        );
        let index = directory.join("index.html");
        self.cache_document(&format!("{base}{}-index.html", filing.accession), &index)
            .await?;
        let attachments = {
            let html = scraper::Html::parse_document(&std::fs::read_to_string(&index)?);
            let rows = scraper::Selector::parse("table.tableFile tr").unwrap();
            let cells = scraper::Selector::parse("td").unwrap();
            let links = scraper::Selector::parse("a[href]").unwrap();
            html.select(&rows)
                .filter_map(|row| {
                    let cells: Vec<_> = row.select(&cells).collect();
                    if cells.len() < 4 {
                        return None;
                    }
                    let kind = cells[3].text().collect::<String>().trim().to_owned();
                    if !(kind.starts_with("EX-99") || kind == filing.form) {
                        return None;
                    }
                    cells[2]
                        .select(&links)
                        .next()
                        .and_then(|link| link.value().attr("href"))
                        .map(|href| (kind, href.to_owned()))
                })
                .collect::<Vec<_>>()
        };
        ensure!(
            !attachments.is_empty(),
            "SEC index contains no relevant documents"
        );
        let mut result = FilingDocuments {
            documents: Vec::new(),
            unavailable: Vec::new(),
        };
        for (kind, href) in attachments {
            let href = href
                .split_once("?doc=")
                .map_or(href.as_str(), |(_, document)| document);
            let url = reqwest::Url::parse(&base)?.join(href)?;
            ensure!(
                url.as_str().starts_with(&base),
                "SEC attachment points outside filing directory"
            );
            let name = url
                .path()
                .rsplit('/')
                .next()
                .context("missing document filename")?;
            ensure!(
                !name.is_empty() && name != "." && name != "..",
                "invalid SEC filename"
            );
            if !(name.ends_with(".htm") || name.ends_with(".html") || name.ends_with(".txt")) {
                result
                    .unavailable
                    .push(format!("{kind} {name}: unsupported attachment format"));
                continue;
            }
            let path = directory.join(name);
            self.cache_document(url.as_str(), &path).await?;
            let content = std::fs::read(&path)?;
            let mut images = Vec::new();
            if !name.ends_with(".txt") {
                let references = {
                    let html = scraper::Html::parse_document(&String::from_utf8_lossy(&content));
                    let mut seen = BTreeSet::new();
                    html.select(&scraper::Selector::parse("img[src]").unwrap())
                        .filter_map(|image| image.value().attr("src").map(str::to_owned))
                        .filter(|reference| seen.insert(reference.clone()))
                        .collect::<Vec<_>>()
                };
                for reference in references {
                    let image = async {
                        let image_url = url.join(&reference)?;
                        ensure!(
                            image_url.as_str().starts_with(&base),
                            "image outside SEC filing directory"
                        );
                        let filename = image_url
                            .path()
                            .rsplit('/')
                            .next()
                            .context("missing image filename")?;
                        ensure!(
                            !filename.is_empty() && filename != "." && filename != "..",
                            "invalid image filename"
                        );
                        let image_path = directory.join(filename);
                        self.cache_document(image_url.as_str(), &image_path).await?;
                        Ok::<_, anyhow::Error>(image_path)
                    }
                    .await;
                    match image {
                        Ok(path) => images.push(path),
                        Err(error) => result
                            .unavailable
                            .push(format!("{kind} {name} image {reference}: {error:#}")),
                    }
                }
            }
            let text = if name.ends_with(".txt") {
                String::from_utf8_lossy(&content).into_owned()
            } else {
                match html2text::from_read(content.as_slice(), 240) {
                    Ok(text) => text,
                    Err(error) => {
                        result.unavailable.push(format!("{kind} {name}: {error}"));
                        if images.is_empty() {
                            continue;
                        }
                        String::new()
                    }
                }
            };
            let text = text
                .lines()
                .map(|line| line.split_whitespace().collect::<Vec<_>>().join(" "))
                .filter(|line| {
                    !line.is_empty()
                        && !line
                            .chars()
                            .all(|character| "─┼┬┴┌┐└┘├┤│ ".contains(character))
                })
                .collect::<Vec<_>>()
                .join("\n");
            if text.trim().len() < 100 && images.is_empty() {
                result
                    .unavailable
                    .push(format!("{kind} {name}: insufficient readable text"));
                continue;
            }
            result.documents.push(FilingDocument {
                name: name.to_owned(),
                kind,
                text,
                images,
            });
        }
        Ok(result)
    }

    async fn cache_document(&self, url: &str, path: &Path) -> Result<()> {
        if path.is_file() {
            return Ok(());
        }
        let mut response = self.get(url).await?;
        let mut file = NamedTempFile::new_in(path.parent().context("missing cache directory")?)?;
        let expected = response.content_length();
        let mut downloaded = 0;
        while let Some(chunk) = response.chunk().await? {
            downloaded += chunk.len() as u64;
            ensure!(
                downloaded <= 128 * 1024 * 1024,
                "SEC document exceeds 128 MiB"
            );
            file.write_all(&chunk)?;
        }
        ensure!(
            expected.is_none_or(|expected| expected == downloaded),
            "incomplete SEC document download"
        );
        file.as_file().sync_all()?;
        file.persist(path)?;
        Ok(())
    }

    pub async fn company_facts_bulk(
        &self,
        directory: &Path,
        mut progress: impl FnMut(u64, Option<u64>),
    ) -> Result<CompanyFactsArchive> {
        for attempt in 0..3 {
            let file = NamedTempFile::new_in(directory)?;
            let mut response = self
                .get("https://www.sec.gov/Archives/edgar/daily-index/xbrl/companyfacts.zip")
                .await?;
            let length = response.content_length();
            let mut downloaded = 0;
            let mut output = tokio::fs::File::from_std(file.reopen()?);
            progress(0, length);
            let result: Result<()> = async {
                while let Some(chunk) = response.chunk().await? {
                    output.write_all(&chunk).await?;
                    downloaded += chunk.len() as u64;
                    progress(downloaded, length);
                }
                output.flush().await?;
                ensure!(
                    length.is_none_or(|length| length == downloaded),
                    "incomplete SEC archive download"
                );
                Ok(())
            }
            .await;
            drop(output);
            match result {
                Ok(()) => return CompanyFactsArchive::open(file),
                Err(error) if attempt < 2 && error.downcast_ref::<reqwest::Error>().is_some() => {
                    log::warn!("SEC archive download interrupted; retrying: {error}");
                    tokio::time::sleep(Duration::from_secs(1 << attempt)).await;
                }
                Err(error) => return Err(error),
            }
        }
        unreachable!()
    }

    pub async fn get(&self, url: &str) -> Result<Response> {
        for attempt in 0..5 {
            {
                let mut next = self.next_request.lock().await;
                tokio::time::sleep_until(*next).await;
                *next = Instant::now() + Duration::from_millis(125);
            }
            match self.http.get(url).send().await {
                Ok(response) if response.status().is_success() => return Ok(response),
                Ok(response)
                    if attempt < 4
                        && (response.status() == StatusCode::TOO_MANY_REQUESTS
                            || response.status().is_server_error()) =>
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
                    let mut next = self.next_request.lock().await;
                    *next = (*next).max(Instant::now() + Duration::from_secs(delay));
                }
                Ok(response) => bail!("SEC returned {} for {url}", response.status()),
                Err(error) if attempt < 4 && (error.is_timeout() || error.is_connect()) => {
                    tokio::time::sleep(Duration::from_secs(1 << attempt)).await;
                }
                Err(error) => return Err(error.into()),
            }
        }
        unreachable!()
    }
}

impl CompanyFactsArchive {
    pub fn open(file: NamedTempFile) -> Result<Self> {
        Ok(Self {
            archive: ZipArchive::new(BufReader::new(file.reopen()?))
                .context("invalid SEC Company Facts archive")?,
            _file: file,
        })
    }

    pub fn company_facts(&mut self, cik: u64) -> Result<Option<Value>> {
        let mut file = match self.archive.by_name(&format!("CIK{cik:010}.json")) {
            Ok(file) => file,
            Err(ZipError::FileNotFound) => return Ok(None),
            Err(error) => return Err(error.into()),
        };
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)?;
        let facts: Value =
            serde_json::from_slice(&bytes).context("invalid SEC Company Facts JSON")?;
        ensure!(
            facts.get("cik").is_none() || company_cik(&facts) == Some(cik),
            "SEC company CIK mismatch"
        );
        ensure!(facts["facts"].is_object(), "missing SEC facts object");
        Ok(Some(facts))
    }
}

pub fn company_cik(facts: &Value) -> Option<u64> {
    facts["cik"]
        .as_u64()
        .or_else(|| facts["cik"].as_str().and_then(|cik| cik.parse().ok()))
}

pub fn retain_periods_since(facts: &mut Value, since: NaiveDate) -> Result<()> {
    for taxonomy in facts["facts"]
        .as_object_mut()
        .context("missing SEC facts object")?
        .values_mut()
    {
        let taxonomy = taxonomy.as_object_mut().context("invalid SEC taxonomy")?;
        for fact in taxonomy.values_mut() {
            let units = fact["units"].as_object_mut().context("missing SEC units")?;
            for observations in units.values_mut() {
                let observations = observations.as_array_mut().context("invalid SEC units")?;
                for observation in observations.iter() {
                    observation["end"]
                        .as_str()
                        .context("missing SEC period end")?
                        .parse::<NaiveDate>()
                        .context("invalid SEC period end")?;
                }
                observations.retain(|observation| {
                    observation["end"]
                        .as_str()
                        .unwrap()
                        .parse::<NaiveDate>()
                        .unwrap()
                        >= since
                });
            }
            units.retain(|_, observations| !observations.as_array().unwrap().is_empty());
        }
        taxonomy.retain(|_, fact| !fact["units"].as_object().unwrap().is_empty());
    }
    facts["facts"]
        .as_object_mut()
        .unwrap()
        .retain(|_, taxonomy| !taxonomy.as_object().unwrap().is_empty());
    Ok(())
}
