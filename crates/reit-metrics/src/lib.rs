pub mod companies;
pub mod debt;
mod derive;
mod models;
pub mod ownership;
mod pdf;
mod presentation;
pub mod publication;
mod reits;
pub mod scan;
pub mod source;
mod table;
pub mod verify;

use std::collections::BTreeMap;

use anyhow::Result;
use chrono::{Datelike, NaiveDate};
use regex::Regex;

pub use models::{AffoSupport, Extraction, Metric, Observation, ReitHistory, Unit};
use table::{Cell, normalize};

pub fn extract(symbol: &str, html: &str, source: &str) -> Result<Extraction> {
    extract_html(
        &companies::company(symbol)?,
        html,
        source,
        false,
        NaiveDate::MIN,
        NaiveDate::MAX,
    )
}

fn extract_html(
    company: &companies::Company,
    html: &str,
    source: &str,
    current_affo_only: bool,
    since: NaiveDate,
    until: NaiveDate,
) -> Result<Extraction> {
    source::validate_url(company, source)?;
    let tables = (company.tables)(html);
    let (mut candidates, report_period) = reported_sections(&company, &tables, source, until)?;
    for observation in company
        .extra_observations
        .map(|extract| extract(html, source))
        .transpose()?
        .unwrap_or_default()
    {
        candidates.push((0, observation));
    }
    for section in &company.sections {
        for observation in section(&tables, source)? {
            candidates.push((usize::MAX, observation));
        }
    }
    candidates.retain(|(_, value)| value.end <= until);
    let report_period = report_period.max(candidates.iter().map(|(_, value)| value.end).max());
    if current_affo_only {
        candidates.retain(|(_, value)| {
            value.metric != Metric::Affo
                || (Some(value.end) == report_period
                    && (scan::is_quarter(value)
                        || (matches!(company.symbol, "STRW" | "FPI") && value.unit == Unit::Total)
                        || company.rules.iter().all(|rule| rule.metric == Metric::Affo)))
        });
    }
    candidates.retain(|(_, value)| value.start >= since);
    let mut result = select(company, candidates)?;
    result.report_period = result.report_period.max(report_period);
    result.source = Some(source.to_owned());
    Ok(result)
}

fn affo_per_share(
    mut company: companies::Company,
    tables: &[table::Table],
    source: &str,
    shares: companies::Rule,
) -> Result<Vec<Observation>> {
    company.rules.retain(|rule| rule.metric == Metric::Affo);
    let reported = select(
        &company,
        reported_sections(&company, tables, source, NaiveDate::MAX)?.0,
    )?
    .observations;
    company.rules = vec![shares];
    let denominators = reported_sections(&company, tables, source, NaiveDate::MAX)?.0;
    Ok(reported
        .iter()
        .filter(|total| {
            total.unit == Unit::Total
                && !reported.iter().any(|value| {
                    value.unit == Unit::PerShare
                        && value.start == total.start
                        && value.end == total.end
                })
        })
        .filter_map(|total| {
            let matching: Vec<_> = denominators
                .iter()
                .map(|(_, value)| value)
                .filter(|value| value.start == total.start && value.end == total.end)
                .map(|value| value.value * value.scale as f64)
                .collect();
            let denominator = *matching.first()?;
            if denominator <= 0.0
                || !denominator.is_finite()
                || matching.iter().any(|value| *value != denominator)
            {
                return None;
            }
            let value = total.value * total.scale as f64 / denominator;
            value.is_finite().then(|| Observation {
                value,
                unit: Unit::PerShare,
                scale: 1,
                label: "Calculated AFFO per diluted share/unit".into(),
                ..total.clone()
            })
        })
        .collect())
}

fn reported_sections(
    company: &companies::Company,
    tables: &[table::Table],
    source: &str,
    until: NaiveDate,
) -> Result<(Vec<(usize, Observation)>, Option<NaiveDate>)> {
    let year = Regex::new(r"\b(20\d{2})\b")?;
    let date = Regex::new(
        r"(?i)(mar(?:ch)?|jun(?:e)?|sep(?:tember)?|dec(?:ember)?)\.?\s*(31|30)|\b(0?[369]|12)/(30|31)/(\d{4}|\d{2})\b|\b(30|31)-(Mar|Jun|Sep|Dec)-(\d{4}|\d{2})\b",
    )?;
    let quarter = Regex::new(r"(?i)^Q([1-4])(?:\s+(20\d{2}))?$")?;
    let range = Regex::new(
        r"(?i)for the period\s*((?:January|February|March|April|May|June|July|August|September|October|November|December)\s+\d{1,2},?\s+20\d{2})\s*to\s*((?:January|February|March|April|May|June|July|August|September|October|November|December)\s+\d{1,2},?\s+20\d{2})",
    )?;
    let footnote = Regex::new(r"\s*(?:\([a-z0-9]+\)|\*|†)")?;
    let mut candidates = BTreeMap::<_, Vec<(usize, Observation)>>::new();
    let mut previous_headers = Vec::new();
    let mut report_period = None;
    for (table_number, table) in tables.iter().enumerate() {
        let heading = table
            .rows
            .iter()
            .take(6)
            .flatten()
            .map(|cell| cell.text.to_lowercase())
            .collect::<Vec<_>>()
            .join(" ");
        if table.is_forecast() {
            previous_headers.clear();
            continue;
        }
        let mut headers = if company
            .continuation
            .is_some_and(|prefix| heading.contains(prefix))
        {
            previous_headers.clone()
        } else {
            Vec::new()
        };
        let mut section = String::new();
        for (row_number, row) in table.rows.iter().enumerate() {
            let Some(first) = row.iter().find(|cell| !cell.text.is_empty()) else {
                continue;
            };
            let is_header = row.iter().any(|cell| {
                year.is_match(&cell.text)
                    || date.is_match(&cell.text)
                    || quarter.is_match(&cell.text)
                    || duration(&cell.text).is_some()
            }) && row
                .iter()
                .filter_map(|cell| number(&cell.text))
                .all(|value| (2000.0..2100.0).contains(&value));
            if is_header {
                if row.iter().any(|cell| duration(&cell.text).is_some()) {
                    headers.clear();
                }
                headers.extend(row.iter().cloned());
                continue;
            }
            let raw_label = if (first.text.eq_ignore_ascii_case("diluted")
                || first.text.eq_ignore_ascii_case("basic and diluted"))
                && section.to_lowercase().contains("per")
            {
                format!("{section} {}", first.text)
            } else {
                first.text.clone()
            };
            let label = normalize(&footnote.replace_all(&raw_label.to_lowercase(), ""));
            if !row.iter().any(|cell| number(&cell.text).is_some()) && !first.text.is_empty() {
                section = first.text.clone();
            }
            if label.starts_with("net income")
                || label.starts_with("net loss")
                || label.starts_with("net earnings")
                || label == "total revenues"
            {
                for cell in row.iter().filter(|cell| number(&cell.text).is_some()) {
                    if let Some((_, end)) =
                        period(&headers, cell.column, &year, &date, &quarter, &range)
                        && end <= until.min(chrono::Utc::now().date_naive())
                        && end.month() % 3 == 0
                        && end.succ_opt().is_some_and(|next| next.day() == 1)
                    {
                        report_period = report_period.max(Some(end));
                    }
                }
            }
            for (priority, rule) in company.rules.iter().enumerate().filter(|(_, rule)| {
                rule.label.is_match(&label)
                    && rule
                        .section
                        .as_ref()
                        .is_none_or(|pattern| pattern.is_match(&section))
            }) {
                let Some(scale) = rule.scale.or(table.amount_scale) else {
                    continue;
                };
                for cell in row.iter().filter(|cell| cell.column > first.column) {
                    let Some(value) = number(&cell.text) else {
                        continue;
                    };
                    let Some((start, end)) =
                        period(&headers, cell.column, &year, &date, &quarter, &range)
                    else {
                        continue;
                    };
                    candidates
                        .entry((rule.metric, start, end))
                        .or_default()
                        .push((
                            priority,
                            Observation {
                                published_on: None,
                                metric: rule.metric,
                                start,
                                end,
                                label: raw_label.clone(),
                                value,
                                unit: rule.unit,
                                scale,
                                source: source.to_owned(),
                                location: format!("table {table_number}, row {row_number}"),
                            },
                        ));
                }
            }
        }
        previous_headers = headers;
    }
    Ok((candidates.into_values().flatten().collect(), report_period))
}

fn same_amount(a: &Observation, b: &Observation) -> bool {
    let (fine, coarse) = if a.scale <= b.scale { (a, b) } else { (b, a) };
    if fine.value * fine.scale as f64 == coarse.value * coarse.scale as f64 {
        return true;
    }
    if fine.unit != Unit::Total || fine.scale == coarse.scale {
        return false;
    }
    let decimals = coarse
        .value
        .to_string()
        .split_once('.')
        .map_or(1, |(_, fraction)| fraction.len().max(1));
    let factor = 10_f64.powi(decimals as i32);
    (fine.value * fine.scale as f64 / coarse.scale as f64 * factor).round()
        == (coarse.value * factor).round()
}

fn select(company: &companies::Company, values: Vec<(usize, Observation)>) -> Result<Extraction> {
    let report_period = values.iter().map(|(_, value)| value.end).max();
    let mut candidates = BTreeMap::<_, Vec<_>>::new();
    for (priority, value) in values {
        candidates
            .entry((value.metric, value.start, value.end, value.unit))
            .or_default()
            .push((priority, value));
    }
    let mut observations = Vec::new();
    for mut values in candidates.into_values() {
        values.sort_by_key(|(priority, value)| (value.unit, *priority, value.scale));
        let (priority, selected) = &values[0];
        if let Some((_, other)) = values.iter().find(|(other_priority, value)| {
            other_priority == priority && !same_amount(selected, value)
        }) {
            return Err(Conflict(format!(
                "{}: conflicting {:?} {:?} for {} through {}: {} × {} ({}) versus {} × {} ({})",
                company.symbol,
                selected.metric,
                selected.unit,
                selected.start,
                selected.end,
                selected.value,
                selected.scale,
                selected.location,
                other.value,
                other.scale,
                other.location
            ))
            .into());
        }
        observations.push(selected.clone());
    }
    Ok(Extraction {
        symbol: company.symbol.to_owned(),
        source: None,
        report_period,
        filed_on: None,
        observations,
    })
}

#[derive(Debug)]
pub(crate) struct Conflict(String);

impl std::fmt::Display for Conflict {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl std::error::Error for Conflict {}

fn number(text: &str) -> Option<f64> {
    let value = text.trim().replace(['$', ',', ' '], "");
    if value.is_empty() || value.contains('%') || matches!(value.as_str(), "-" | "—" | "–") {
        return None;
    }
    let parsed = if value.starts_with('(') && value.ends_with(')') {
        -value[1..value.len() - 1].parse::<f64>().ok()?
    } else {
        value.parse::<f64>().ok()?
    };
    parsed.is_finite().then_some(parsed)
}

fn duration(text: &str) -> Option<u32> {
    let text = text.to_lowercase().replace('-', " ");
    if text.contains("three months")
        || text.contains("quarter ended")
        || text.contains("quarters ended")
    {
        Some(3)
    } else if text.contains("six months") {
        Some(6)
    } else if text.contains("nine months") {
        Some(9)
    } else if text.contains("year ended")
        || text.contains("years ended")
        || text.contains("twelve months")
    {
        Some(12)
    } else {
        None
    }
}

fn period(
    headers: &[Cell],
    column: usize,
    year: &Regex,
    date: &Regex,
    quarter: &Regex,
    range: &Regex,
) -> Option<(NaiveDate, NaiveDate)> {
    let aligned: Vec<_> = headers
        .iter()
        .rev()
        .filter(|cell| cell.covers(column))
        .collect();
    if let Some(period) = aligned.iter().find_map(|cell| range.captures(&cell.text)) {
        return Some((
            NaiveDate::parse_from_str(&period[1].replace(',', ""), "%B %d %Y").ok()?,
            NaiveDate::parse_from_str(&period[2].replace(',', ""), "%B %d %Y").ok()?,
        ));
    }
    if let Some(period) = aligned.iter().find_map(|cell| quarter.captures(&cell.text)) {
        let month = period[1].parse::<u32>().ok()? * 3;
        let year = period
            .get(2)
            .and_then(|value| value.as_str().parse().ok())
            .or_else(|| {
                aligned.iter().find_map(|cell| {
                    year.captures(&cell.text)
                        .and_then(|value| value[1].parse().ok())
                })
            })?;
        let start = NaiveDate::from_ymd_opt(year, month - 2, 1)?;
        return Some((
            start,
            start
                .checked_add_months(chrono::Months::new(3))?
                .pred_opt()?,
        ));
    }
    let month_day = aligned.iter().find_map(|cell| date.captures(&cell.text))?;
    let year: i32 = if let Some(value) = month_day.get(5).or_else(|| month_day.get(8)) {
        let parsed = value.as_str().parse::<i32>().ok()?;
        if parsed < 100 { 2000 + parsed } else { parsed }
    } else {
        aligned.iter().find_map(|cell| {
            year.captures(&cell.text)
                .map(|value| value[1].parse().unwrap())
        })?
    };
    let (month, day) = if let Some(named) = month_day.get(1) {
        (
            match named.as_str().to_lowercase().as_str() {
                "mar" | "march" => 3,
                "jun" | "june" => 6,
                "sep" | "september" => 9,
                "dec" | "december" => 12,
                _ => return None,
            },
            month_day[2].parse().ok()?,
        )
    } else if let Some(named) = month_day.get(7) {
        (
            match named.as_str().to_lowercase().as_str() {
                "mar" => 3,
                "jun" => 6,
                "sep" => 9,
                "dec" => 12,
                _ => return None,
            },
            month_day[6].parse().ok()?,
        )
    } else {
        (month_day[3].parse().ok()?, month_day[4].parse().ok()?)
    };
    let months = aligned.iter().find_map(|cell| duration(&cell.text))?;
    let end = NaiveDate::from_ymd_opt(year, month, day)?;
    if end.month() < months {
        return None;
    }
    Some((NaiveDate::from_ymd_opt(year, month + 1 - months, 1)?, end))
}

pub fn extract_file(symbol: &str, path: &std::path::Path, source: &str) -> Result<Extraction> {
    extract_document(
        &companies::company(symbol)?,
        path,
        source,
        false,
        NaiveDate::MIN,
        NaiveDate::MAX,
    )
}

fn extract_document(
    company: &companies::Company,
    path: &std::path::Path,
    source: &str,
    current_affo_only: bool,
    since: NaiveDate,
    until: NaiveDate,
) -> Result<Extraction> {
    source::validate_url(company, source)?;
    let bytes = std::fs::read(path)?;
    if bytes.starts_with(b"%PDF-") {
        let rules = company.pdf.as_ref().ok_or_else(|| {
            anyhow::anyhow!("{}: PDF adapter has not been researched", company.symbol)
        })?;
        let text = pdf::text(path)?;
        let mut observations = pdf::extract(rules, &text, source)?;
        if let Some(extract) = company.pdf_observations {
            observations = extract(&text, source, observations)?;
        }
        observations.retain(|value| value.end <= until);
        if current_affo_only {
            let end = observations.iter().map(|value| value.end).max();
            observations.retain(|value| {
                value.metric != Metric::Affo || (Some(value.end) == end && scan::is_quarter(value))
            });
        }
        observations.retain(|value| value.start >= since);
        let mut result = select(
            company,
            observations
                .into_iter()
                .map(|value| (usize::from(value.label.starts_with("Calculated ")), value))
                .collect(),
        )?;
        result.source = Some(source.to_owned());
        Ok(result)
    } else {
        extract_html(
            company,
            std::str::from_utf8(&bytes)?,
            source,
            current_affo_only,
            since,
            until,
        )
    }
}

pub use scan::{Fetch, Report, fetch};
