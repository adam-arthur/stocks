use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "PLD",
        1045609,
        &["www.prologis.com", "ir.prologis.com"][..],
        vec![
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^nareit defined ffo per (?:common |diluted )?share$",
                1,
            ),
            (
                Metric::Ffo,
                Unit::Total,
                r"^nareit defined ffo attributable to common stockholders/unitholders$",
                1000,
            ),
        ],
        None,
    ))?;
    company.extra_observations = Some(extract);
    company.official_url = |url| {
        url.host_str() == Some("d1io3yog0oux5.cloudfront.net")
            && url.path().split('/').nth(2) == Some("prologis")
    };
    company.supplements =
        Some(|sources, company, since, _| Box::pin(supplements(sources, company, since)));
    company.pdf = Some(crate::pdf::Rules {
        page: pdf_page,
        total_scale: 1000,
        metric: pdf_metric,
        current_only: |_, _, _, _| false,
    });
    Ok(company)
}

use chrono::NaiveDate;
use regex::Regex;
use scraper::Html;

use crate::{Observation, table::normalize};

pub fn extract(html: &str, source: &str) -> Result<Vec<Observation>> {
    let date = Regex::new(r"(?i)(March|June|September|December)\s+(31|30)")?;
    let years = Regex::new(r"\b((?:20\d{2}\s+){1,3}20\d{2})\b")?;
    let values = Regex::new(r"^\s*\$?\s*([+-]?\d[\d,]*(?:\.\d+)?)")?;
    let labels = [(
        Metric::Ffo,
        Unit::Total,
        Regex::new(r"NAREIT defined FFO attributable to common stockholders/unitholders\*?\s+\$")?,
    )];
    let mut result = Vec::new();
    for (block, text) in Html::parse_document(html).root_element().text().enumerate() {
        let text = normalize(text);
        let reconciliation = text.starts_with("Reconciliations of Net Earnings to FFO");
        if !reconciliation {
            continue;
        }
        let Some(header_start) = text.find("Three Months Ended") else {
            continue;
        };
        let Some(year_match) = years.captures(&text[header_start..]) else {
            continue;
        };
        let header = &text[header_start..header_start + year_match.get(0).unwrap().start()];
        let Some(date_match) = date.captures(header) else {
            continue;
        };
        let month = match date_match[1].to_lowercase().as_str() {
            "march" => 3,
            "june" => 6,
            "september" => 9,
            "december" => 12,
            _ => continue,
        };
        let day = date_match[2].parse::<u32>()?;
        if date
            .captures_iter(header)
            .any(|other| other[1] != date_match[1] || other[2] != date_match[2])
        {
            continue;
        }
        let years: Vec<i32> = year_match[1]
            .split_whitespace()
            .map(str::parse)
            .collect::<std::result::Result<_, _>>()?;
        if !matches!(years.len(), 2 | 4)
            || years[0] != years[1] + 1
            || (years.len() == 4 && years[..2] != years[2..])
        {
            continue;
        }
        let second_duration = if header.contains("Six Months") {
            6
        } else if header.contains("Nine Months") {
            9
        } else if header.contains("Twelve Months") {
            12
        } else {
            0
        };
        if years.len() == 4 && second_duration == 0 {
            continue;
        }
        let body = &text[header_start + year_match.get(0).unwrap().end()..];
        for (metric, unit, pattern) in &labels {
            if *unit == Unit::Total
                && (!reconciliation
                    || !text[..header_start + year_match.get(0).unwrap().end()]
                        .contains("in thousands"))
            {
                continue;
            }
            for found in pattern.find_iter(body) {
                append(
                    &mut result,
                    &values,
                    &body[found.end()..],
                    &years,
                    (month, day, second_duration),
                    Observation {
                        metric: *metric,
                        start: NaiveDate::MIN,
                        end: NaiveDate::MIN,
                        label: found.as_str().trim_end_matches(['*', '$', ' ']).to_owned(),
                        value: 0.0,
                        unit: *unit,
                        scale: if *unit == Unit::Total { 1000 } else { 1 },
                        source: source.to_owned(),
                        location: format!(
                            "text block {block}: {}",
                            if reconciliation {
                                "FFO reconciliation"
                            } else {
                                "per-share calculation"
                            }
                        ),
                    },
                );
            }
        }
    }
    Ok(result)
}

fn append(
    result: &mut Vec<Observation>,
    pattern: &Regex,
    text: &str,
    years: &[i32],
    dates: (u32, u32, u32),
    observation: Observation,
) {
    let (month, day, second_duration) = dates;
    let mut remaining = text;
    let mut parsed = Vec::new();
    while let Some(value) = pattern.captures(remaining) {
        let Ok(number) = value[1].replace(',', "").parse::<f64>() else {
            return;
        };
        parsed.push(number);
        remaining = &remaining[value.get(0).unwrap().end()..];
    }
    if parsed.len() != years.len() {
        return;
    }
    for (index, (year, value)) in years.iter().zip(parsed).enumerate() {
        let months = if index < 2 { 3 } else { second_duration };
        if months > month {
            return;
        }
        let Some(start) = NaiveDate::from_ymd_opt(*year, month + 1 - months, 1) else {
            return;
        };
        let Some(end) = NaiveDate::from_ymd_opt(*year, month, day) else {
            return;
        };
        result.push(Observation {
            start,
            end,
            value,
            ..observation.clone()
        });
    }
}

pub async fn supplements(
    sources: &crate::source::Sources,
    company: &Company,
    since: chrono::NaiveDate,
) -> Result<Vec<(chrono::NaiveDate, String)>> {
    anyhow::ensure!(
        company.symbol == "PLD",
        "no corporate archive adapter for {}",
        company.symbol
    );
    let html = scraper::Html::parse_document(&std::fs::read_to_string(
        sources
            .download_inner(
                company,
                "https://ir.prologis.com/financials/quarterly-results",
                true,
            )
            .await?,
    )?);
    let pattern = regex::Regex::new(r"^Supplemental Financial Report Q([1-4]) (20\d{2}) PDF$")?;
    let mut result = Vec::new();
    for link in html.select(&scraper::Selector::parse("a[aria-label][href]").unwrap()) {
        let Some(label) = pattern.captures(link.value().attr("aria-label").unwrap()) else {
            continue;
        };
        let month = label[1].parse::<u32>()? * 3;
        let end = chrono::NaiveDate::from_ymd_opt(
            label[2].parse()?,
            month,
            if month == 6 || month == 9 { 30 } else { 31 },
        )
        .ok_or_else(|| anyhow::anyhow!("invalid archive period"))?;
        if end >= since {
            let url = link.value().attr("href").unwrap();
            crate::source::validate_url(company, url)?;
            result.push((end, url.to_owned()));
        }
    }
    anyhow::ensure!(
        !result.is_empty(),
        "corporate archive contains no matching supplements"
    );
    result.sort();
    result.dedup();
    Ok(result)
}

pub fn pdf_page(text: &str) -> bool {
    text.contains("reconciliationsofnetearningstoffo")
}

pub fn pdf_metric(label: &str, page: &str) -> Option<(Metric, Unit)> {
    match label.trim_end_matches('*') {
        "nareitdefinedffoattributabletocommonstockholders/unitholders"
        | "subtotal-nareitdefinedffo"
            if pdf_page(page) && page.contains("inthousands") =>
        {
            Some((Metric::Ffo, Unit::Total))
        }
        _ => None,
    }
}
