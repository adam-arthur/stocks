use std::collections::BTreeMap;

use anyhow::{Context, Result, ensure};
use chrono::{Datelike, NaiveDate};
use serde::Serialize;

use crate::{
    Extraction, Metric, Observation, Unit, companies::company, extract_file, source::Sources,
};

#[derive(Serialize)]
pub struct Report {
    pub affo_support: crate::AffoSupport,
    pub symbol: String,
    pub report_period: Option<NaiveDate>,
    pub observations: Vec<Observation>,
    pub documents: Vec<Extraction>,
    pub ownership: BTreeMap<NaiveDate, crate::ownership::Ownership>,
}

pub enum Fetch {
    All { since: NaiveDate },
    Latest { since: NaiveDate, unit: Unit },
}

pub async fn fetch(symbol: &str, mode: Fetch, sources: &Sources) -> Result<Report> {
    let (since, unit) = match mode {
        Fetch::All { since } => (since, None),
        Fetch::Latest { since, unit } => (since, Some(unit)),
    };
    let latest = unit.is_some();
    let company = company(symbol)?;
    let mut documents = Vec::new();
    let mut publication_errors = BTreeMap::new();
    let mut ownership = BTreeMap::new();
    let mut filings = sources
        .sec
        .earnings_filings(company.cik, since)
        .await?
        .into_iter()
        .filter(|filing| {
            company.filings.contains(&filing.form.as_str())
                || (crate::ownership::supports(symbol)
                    && matches!(filing.form.as_str(), "10-Q" | "10-Q/A" | "10-K" | "10-K/A"))
        })
        .collect::<Vec<_>>();
    if company.additional_items.is_some()
        || company.filings.iter().any(|form| {
            !matches!(
                *form,
                "8-K" | "8-K/A" | "10-Q" | "10-Q/A" | "10-K" | "10-K/A"
            )
        })
    {
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
        let document_start = documents.len();
        match sources
            .filing_urls(&company, &filing.accession, &filing.form)
            .await
        {
            Ok(mut urls) if !urls.is_empty() => {
                urls.sort_by_key(|url| url.to_ascii_lowercase().ends_with(".pdf"));
                for url in urls {
                    match sources.download(&company, &url).await.and_then(|path| {
                        collect_ownership(
                            symbol,
                            &path,
                            &url,
                            Some(filing.filed_on),
                            &mut ownership,
                        );
                        if company.filings.contains(&filing.form.as_str()) {
                            extract_file(symbol, &path, &url).map(Some)
                        } else {
                            Ok(None)
                        }
                    }) {
                        Ok(Some(mut result)) => {
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
                            if !matches!(
                                filing.form.as_str(),
                                "8-K"
                                    | "8-K/A"
                                    | "6-K"
                                    | "6-K/A"
                                    | "10-Q"
                                    | "10-Q/A"
                                    | "10-K"
                                    | "10-K/A"
                            ) {
                                for value in &mut result.observations {
                                    value.published_on = Some(filing.filed_on);
                                }
                            }
                            result.filed_on = Some(filing.filed_on);
                            documents.push(result);
                        }
                        Ok(None) => {}
                        Err(error) if error.is::<crate::Conflict>() => return Err(error),
                        Err(error) => log::warn!("{url}: {error:#}"),
                    }
                }
            }
            Ok(_) => log::debug!("{}: no earnings exhibits", filing.accession),
            Err(error) => log::warn!("{}: {error:#}", filing.accession),
        }
        if documents[document_start..]
            .iter()
            .any(|document| !document.observations.is_empty())
            && (filing.form.starts_with("8-K") || filing.form.starts_with("6-K"))
        {
            let published_on = sources.publication_date(&company, &filing).await;
            for document in &mut documents[document_start..] {
                for value in &mut document.observations {
                    match &published_on {
                        Ok(date) => value.published_on = Some(*date),
                        Err(error) => {
                            publication_errors.insert(value.source.clone(), format!("{error:#}"));
                        }
                    }
                }
            }
        }
    }
    if let Some(supplements) = company.supplements {
        let supplements = supplements(
            sources,
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
                    supplements.sort_by_key(|(end, _, _)| std::cmp::Reverse(*end));
                    supplements.truncate(1);
                }
                for (end, url, published_on) in supplements {
                    if documents
                        .iter()
                        .any(|document| document.source.as_deref() == Some(url.as_str()))
                    {
                        continue;
                    }
                    log::debug!("{symbol}: official supplement {end}");
                    match sources.download(&company, &url).await.and_then(|path| {
                        collect_ownership(symbol, &path, &url, None, &mut ownership);
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
                            if let Some(date) = published_on {
                                ensure!(
                                    date >= end,
                                    "supplement publication date precedes report period"
                                );
                                for value in &mut result.observations {
                                    value.published_on = Some(date);
                                }
                            }
                            documents.push(result);
                        }
                        Err(error) if error.is::<crate::Conflict>() => return Err(error),
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
    let mut observations = contiguous_series(
        original_series(&documents, since, company.affo_comparatives)?
            .into_iter()
            .filter(|value| {
                value.metric == Metric::Affo || unit.is_none_or(|unit| value.unit == unit)
            })
            .collect(),
        report_period,
    );
    if latest {
        observations.retain(|value| Some(value.end) == report_period);
    }
    for value in &observations {
        let date = value.published_on.with_context(|| {
            format!(
                "{symbol}: missing Item 2.02 announcement date for quarter ending {} ({}): {}",
                value.end,
                value.source,
                publication_errors
                    .get(&value.source)
                    .map_or("no associated Item 2.02 filing", String::as_str)
            )
        })?;
        ensure!(
            date > value.end,
            "{symbol}: announcement date {date} must follow quarter ending {}",
            value.end
        );
    }
    Ok(Report {
        affo_support: company.affo_support,
        symbol: symbol.to_owned(),
        report_period,
        observations,
        documents,
        ownership,
    })
}

fn original_series(
    documents: &[Extraction],
    since: NaiveDate,
    affo_comparatives: bool,
) -> Result<Vec<Observation>> {
    let mut periods = BTreeMap::<_, Vec<_>>::new();
    for document in documents {
        for value in document.observations.iter().filter(|value| {
            value.start >= since
                && (value.metric != Metric::Affo
                    || affo_comparatives
                    || document.report_period == Some(value.end))
        }) {
            periods
                .entry((value.metric, value.unit, value.start, value.end))
                .or_default()
                .push((document, value));
        }
    }
    let mut observations = Vec::new();
    for ((_, _, _, end), mut candidates) in periods {
        candidates.sort_by_key(|(document, value)| {
            (
                document.report_period != Some(end),
                value.label.starts_with("Calculated "),
                document.filed_on.unwrap_or(NaiveDate::MAX),
            )
        });
        let (document, selected) = candidates[0];
        for (other_document, other) in &candidates[1..] {
            if other_document.report_period == document.report_period
                && other_document.filed_on == document.filed_on
                && other.published_on == selected.published_on
                && other.label.starts_with("Calculated ")
                    == selected.label.starts_with("Calculated ")
                && other.value * other.scale as f64 != selected.value * selected.scale as f64
            {
                return Err(crate::Conflict(format!(
                    "{}: conflicting {:?} {:?} for {} through {} in {} and {}",
                    document.symbol,
                    selected.metric,
                    selected.unit,
                    selected.start,
                    selected.end,
                    selected.source,
                    other.source
                ))
                .into());
            }
        }
        observations.push(selected.clone());
    }
    Ok(observations)
}

pub fn contiguous_series(values: Vec<Observation>, latest: Option<NaiveDate>) -> Vec<Observation> {
    let mut metrics = BTreeMap::<_, Vec<_>>::new();
    for value in values.into_iter().filter(is_quarter) {
        metrics
            .entry((value.metric, value.unit))
            .or_default()
            .push(value);
    }
    let mut selected = BTreeMap::<_, Vec<Observation>>::new();
    for values in metrics
        .into_values()
        .map(|mut values| {
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
        .filter(|values| !values.is_empty())
    {
        let current = selected
            .entry((
                values[0].metric,
                (values[0].metric == Metric::Affo).then_some(values[0].unit),
            ))
            .or_default();
        if values.len() > current.len() {
            *current = values;
        }
    }
    selected.into_values().flatten().collect()
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

fn collect_ownership(
    symbol: &str,
    path: &std::path::Path,
    source: &str,
    filed: Option<NaiveDate>,
    result: &mut BTreeMap<NaiveDate, crate::ownership::Ownership>,
) {
    if !crate::ownership::supports(symbol) {
        return;
    }
    let Ok(html) = std::fs::read_to_string(path) else {
        return;
    };
    match crate::ownership::extract(symbol, &html, source, filed) {
        Ok(values) => {
            for (end, value) in values {
                crate::ownership::merge(result.entry(end).or_default(), value);
            }
        }
        Err(error) => log::warn!("{symbol}: ownership extraction failed for {source}: {error:#}"),
    }
}
