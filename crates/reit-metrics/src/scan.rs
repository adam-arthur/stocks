use std::{collections::BTreeMap, path::Path};

use anyhow::{Result, ensure};
use chrono::{Datelike, NaiveDate};
use serde::Serialize;

use crate::{Extraction, Observation, companies::company, extract_file, source::Sources};

#[derive(Serialize)]
pub struct Report {
    pub symbol: String,
    pub report_period: Option<NaiveDate>,
    pub observations: Vec<Observation>,
    pub documents: Vec<Extraction>,
}

pub enum Fetch {
    All { since: NaiveDate },
    Latest { since: NaiveDate },
}

pub async fn fetch(symbol: &str, mode: Fetch, cache: &Path) -> Result<Report> {
    let (since, latest) = match mode {
        Fetch::All { since } => (since, false),
        Fetch::Latest { since } => (since, true),
    };
    let company = company(symbol)?;
    let sources = Sources::new(cache)?;
    let mut documents = Vec::new();
    let mut filings = sec_client::SecClient::from_env()?
        .earnings_filings(company.cik, since)
        .await?
        .into_iter()
        .filter(|filing| company.filings.contains(&filing.form.as_str()))
        .collect::<Vec<_>>();
    if company.additional_items.is_some() {
        for filing in sources.additional_filings(&company, since).await? {
            if !filings
                .iter()
                .any(|existing| existing.accession == filing.accession)
            {
                filings.push(filing);
            }
        }
    }
    filings.sort_by_key(|filing| std::cmp::Reverse(filing.filed_on));
    for filing in filings {
        if latest
            && documents
                .iter()
                .flat_map(|document: &Extraction| &document.observations)
                .filter(|value| is_quarter(value) && value.start >= since)
                .map(|value| value.end)
                .max()
                .is_some_and(|end| filing.filed_on <= end)
        {
            break;
        }
        log::debug!("{symbol}: {} {}", filing.filed_on, filing.accession);
        match sources
            .filing_urls(&company, &filing.accession, &filing.form)
            .await
        {
            Ok(urls) if !urls.is_empty() => {
                for url in urls {
                    match sources
                        .download(&company, &url)
                        .await
                        .and_then(|path| extract_file(symbol, &path, &url))
                    {
                        Ok(mut result) => {
                            result
                                .observations
                                .retain(|value| value.end <= filing.filed_on);
                            if result
                                .report_period
                                .is_some_and(|end| end > filing.filed_on)
                            {
                                result.report_period =
                                    result.observations.iter().map(|value| value.end).max();
                            }
                            result.filed_on = Some(filing.filed_on);
                            documents.push(result);
                        }
                        Err(error) => log::warn!("{url}: {error:#}"),
                    }
                }
            }
            Ok(_) => log::debug!("{}: no earnings exhibits", filing.accession),
            Err(error) => log::warn!("{}: {error:#}", filing.accession),
        }
    }
    if let Some(supplements) = company.supplements {
        let supplements = supplements(
            &sources,
            &company,
            since,
            documents
                .iter()
                .filter_map(|document| document.report_period)
                .max()
                .unwrap_or(since),
        )
        .await;
        match supplements {
            Ok(mut supplements) => {
                if latest {
                    supplements.sort_by_key(|(end, _)| std::cmp::Reverse(*end));
                    supplements.truncate(1);
                }
                for (end, url) in supplements {
                    if documents
                        .iter()
                        .filter(|document| document.report_period == Some(end))
                        .flat_map(|document| &document.observations)
                        .any(|value| value.end == end && is_quarter(value))
                    {
                        continue;
                    }
                    log::debug!("{symbol}: official supplement {end}");
                    match sources.download(&company, &url).await.and_then(|path| {
                        let result = extract_file(symbol, &path, &url)?;
                        ensure!(
                            !company.supplement_period_required
                                || result.report_period == Some(end),
                            "corporate release does not match expected period {end}"
                        );
                        Ok(result)
                    }) {
                        Ok(mut result) => {
                            result.report_period = Some(end);
                            documents.push(result);
                        }
                        Err(error) => log::warn!("{url}: {error:#}"),
                    }
                }
            }
            Err(error) => log::warn!("corporate archive: {error:#}"),
        }
    }
    ensure!(
        !documents.is_empty(),
        "no source documents could be extracted"
    );
    if let Some(derive) = company.derive {
        documents.extend(derive(&documents));
    }
    let report_period = documents
        .iter()
        .filter_map(|document| document.report_period)
        .max();
    let mut observations = contiguous_series(original_series(&documents, since), report_period);
    if latest {
        observations.retain(|value| Some(value.end) == report_period);
    }
    Ok(Report {
        symbol: symbol.to_owned(),
        report_period,
        observations,
        documents,
    })
}

fn original_series(documents: &[Extraction], since: NaiveDate) -> Vec<Observation> {
    let mut periods = BTreeMap::<_, Vec<_>>::new();
    for document in documents {
        for value in document
            .observations
            .iter()
            .filter(|value| value.start >= since)
        {
            periods
                .entry((value.metric, value.start, value.end))
                .or_default()
                .push((document, value));
        }
    }
    let mut observations = Vec::new();
    for ((_, _, end), mut candidates) in periods {
        candidates.sort_by_key(|(document, value)| {
            (
                document.report_period != Some(end),
                value.unit,
                document.filed_on.unwrap_or(NaiveDate::MAX),
            )
        });
        observations.push(candidates[0].1.clone());
    }
    observations
}

pub fn contiguous_series(values: Vec<Observation>, latest: Option<NaiveDate>) -> Vec<Observation> {
    let mut metrics = BTreeMap::<_, Vec<_>>::new();
    for value in values.into_iter().filter(is_quarter) {
        metrics.entry(value.metric).or_default().push(value);
    }
    metrics
        .into_values()
        .flat_map(|mut values| {
            values.sort_by_key(|value| (value.end, value.start));
            if values.last().is_none_or(|value| Some(value.end) != latest) {
                return Vec::new();
            }
            let start = values
                .windows(2)
                .rposition(|pair| pair[0].end.succ_opt() != Some(pair[1].start))
                .map_or(0, |index| index + 1);
            values.drain(..start);
            values
        })
        .collect()
}

pub fn is_quarter(value: &Observation) -> bool {
    let start = NaiveDate::from_ymd_opt(value.end.year(), value.end.month(), 1)
        .and_then(|end| end.checked_sub_months(chrono::Months::new(2)))
        .unwrap();
    value.start >= start
        && value.start <= value.end
        && start
            .checked_add_months(chrono::Months::new(3))
            .and_then(|date| date.pred_opt())
            == Some(value.end)
}
