use std::collections::BTreeMap;

use anyhow::{Context, Result, ensure};
use chrono::{Datelike, NaiveDate};
use serde::Serialize;

use crate::{
    Extraction, Metric, Observation, Unit, companies::company, extract_document, source::Sources,
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
    if matches!(symbol, "FRMI" | "SRG" | "PW" | "SAFE") {
        let filing = filings
            .iter()
            .max_by_key(|filing| (&filing.filed_on, &filing.accession))
            .context("missing earnings filing")?;
        let mut urls = sources
            .filing_urls(&company, &filing.accession, &filing.form)
            .await?;
        ensure!(!urls.is_empty(), "{symbol}: missing earnings exhibits");
        if symbol == "SAFE" {
            let release = filings
                .iter()
                .filter(|f| f.form.starts_with("8-K") != filing.form.starts_with("8-K"))
                .max_by_key(|f| f.filed_on)
                .context("missing corresponding Safehold earnings filing")?;
            let exhibits = sources
                .filing_urls(&company, &release.accession, &release.form)
                .await?;
            ensure!(!exhibits.is_empty(), "missing Safehold earnings exhibits");
            urls.extend(exhibits);
            urls.sort();
            urls.dedup();
        }
        let mut reports_metrics = false;
        let metric = regex::Regex::new(
            r"(?i)\b(?:affo|ffo|fad|funds from operations|funds available for distribution)\b",
        )?;
        for url in urls {
            let path = sources.download(&company, &url).await?;
            let bytes = std::fs::read(&path)?;
            if symbol == "PW" {
                let tables = crate::table::joined_parentheses(std::str::from_utf8(&bytes)?);
                let labels = tables
                    .iter()
                    .flat_map(|table| &table.rows)
                    .filter(|row| row.iter().any(|cell| crate::number(&cell.text).is_some()))
                    .filter_map(|row| row.iter().find(|cell| !cell.text.is_empty()))
                    .map(|cell| cell.text.to_lowercase())
                    .collect::<Vec<_>>();
                ensure!(
                    labels.iter().any(|label| label.starts_with("core ffo"))
                        && !labels.iter().any(|label| metric.is_match(label)
                            && !label.contains("core ffo")
                            && !label.contains("core funds from operations")),
                    "{symbol}: expected only Core FFO in the reported figures: {url}"
                );
                continue;
            }
            let text = if bytes.starts_with(b"%PDF-") {
                crate::pdf::text(&path)?
            } else {
                let document = scraper::Html::parse_document(std::str::from_utf8(&bytes)?);
                document.root_element().text().collect::<Vec<_>>().join(" ")
            };
            if symbol == "SAFE" && metric.is_match(&crate::table::normalize(&text)) {
                reports_metrics = true;
                continue;
            }
            ensure!(
                !metric.is_match(&crate::table::normalize(&text)),
                "{symbol}: earnings exhibit mentions REIT metrics and requires extraction rules: {url}"
            );
        }
        if !reports_metrics {
            return Ok(Report {
                affo_support: crate::AffoSupport::Unavailable,
                symbol: symbol.to_owned(),
                report_period: None,
                observations: Vec::new(),
                documents,
                ownership,
            });
        }
    }
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
                        )?;
                        if company.filings.contains(&filing.form.as_str()) {
                            extract_document(
                                &company,
                                &path,
                                &url,
                                !company.affo_comparatives,
                                since,
                                filing.filed_on,
                            )
                            .map(Some)
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
                            if !matches!(filing.form.as_str(), "8-K" | "8-K/A" | "6-K" | "6-K/A") {
                                for value in &mut result.observations {
                                    value.published_on = Some(filing.filed_on);
                                }
                            }
                            result.filed_on = Some(filing.filed_on);
                            documents.push(result);
                        }
                        Ok(None) => {}
                        Err(error)
                            if error.to_string() == "unsupported document format"
                                || error
                                    .to_string()
                                    .ends_with("PDF adapter has not been researched") =>
                        {
                            log::warn!("{url}: {error:#}")
                        }
                        Err(error) => return Err(error.context(url)),
                    }
                }
            }
            Ok(_) => log::debug!("{}: no earnings exhibits", filing.accession),
            Err(error) => return Err(error.context(format!("filing index {}", filing.accession))),
        }
        if documents[document_start..]
            .iter()
            .any(|document| !document.observations.is_empty())
            && (filing.form.starts_with("8-K") || filing.form.starts_with("6-K"))
        {
            let mut published_on = sources.publication_date(&company, &filing).await;
            if filing.form.starts_with("8-K")
                && (published_on.is_err()
                    || documents[document_start..]
                        .iter()
                        .filter_map(|doc| doc.report_period)
                        .max()
                        .is_some_and(|end| published_on.as_ref().is_ok_and(|date| *date <= end)))
            {
                let mut dates = std::collections::BTreeSet::new();
                for url in documents[document_start..]
                    .iter()
                    .filter_map(|doc| doc.source.as_ref())
                    .filter(|url| !url.to_ascii_lowercase().ends_with(".pdf"))
                {
                    let html = sources.get(&company, url).await?;
                    if let Some(date) = crate::publication::exhibit_date(&html, filing.filed_on)
                        .with_context(|| format!("reading dateline in {url}"))?
                    {
                        dates.insert(date);
                    }
                }
                ensure!(
                    dates.len() <= 1,
                    "{symbol}: conflicting earnings exhibit datelines in {}",
                    filing.accession
                );
                if let Some(date) = dates.pop_first() {
                    published_on = Ok(date);
                }
            }
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
                        collect_ownership(symbol, &path, &url, None, &mut ownership)?;
                        let result = extract_document(
                            &company,
                            &path,
                            &url,
                            !company.affo_comparatives,
                            since,
                            published_on.unwrap_or(end),
                        )?;
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
                        Err(error) => return Err(error.context(url)),
                    }
                }
            }
            Err(error) => return Err(error.context("corporate earnings archive")),
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
                "{symbol}: missing verified publication date for quarter ending {} ({}): {}",
                value.end,
                value.source,
                publication_errors
                    .get(&value.source)
                    .map_or("no associated dated filing", String::as_str)
            )
        })?;
        ensure!(
            date > value.end,
            "{symbol}: announcement date {date} must follow quarter ending {} in {}",
            value.end,
            value.source
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
                value.scale,
            )
        });
        let (document, selected) = candidates[0];
        for (other_document, other) in &candidates[1..] {
            if other_document.report_period == document.report_period
                && other_document.filed_on == document.filed_on
                && other.published_on == selected.published_on
                && other.label.starts_with("Calculated ")
                    == selected.label.starts_with("Calculated ")
                && !crate::same_amount(selected, other)
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
    let mut affo = Vec::new();
    for value in values.into_iter().filter(is_quarter) {
        if value.metric == Metric::Affo {
            affo.push(value);
            continue;
        }
        metrics.entry(value.unit).or_default().push(value);
    }
    let mut selected = Vec::new();
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
        if values.len() > selected.len() {
            selected = values;
        }
    }
    let periods: BTreeMap<_, _> = selected.iter().map(|v| (v.end, v.start)).collect();
    affo.retain(|v| {
        v.start.checked_add_months(chrono::Months::new(3)) == v.end.succ_opt()
            && periods.get(&v.end).is_none_or(|&start| start == v.start)
    });
    let dates: BTreeMap<_, _> = affo.iter().map(|v| (v.end, v.start)).collect();
    let mut end = latest;
    while let Some(start) = end.and_then(|date| dates.get(&date)) {
        end = start.pred_opt();
    }
    selected.extend(
        affo.into_iter()
            .filter(|v| Some(v.end) > end && Some(v.end) <= latest),
    );
    selected
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
) -> Result<()> {
    if !crate::ownership::supports(symbol) {
        return Ok(());
    }
    let bytes = std::fs::read(path)?;
    if bytes.starts_with(b"%PDF-") {
        return Ok(());
    }
    let html = std::str::from_utf8(&bytes)?;
    for (end, value) in crate::ownership::extract(symbol, html, source, filed)? {
        crate::ownership::merge(result.entry(end).or_default(), value);
    }
    Ok(())
}
