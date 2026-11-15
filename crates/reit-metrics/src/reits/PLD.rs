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
    company.affo_support = crate::AffoSupport::Reported;
    company.affo_comparatives = true;
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
    company.pdf_observations = Some(pdf_observations);
    company.debt = Some(debt);
    company.ownership = Some(crate::ownership::Rules::default());
    Ok(company)
}

use chrono::NaiveDate;
use regex::Regex;
use scraper::Html;

use crate::{Observation, table::normalize};

pub fn extract(html: &str, source: &str) -> Result<Vec<Observation>> {
    extract_blocks(
        Html::parse_document(html)
            .root_element()
            .text()
            .enumerate()
            .map(|(index, text)| (format!("text block {index}"), normalize(text)))
            .collect(),
        source,
    )
}

fn pdf_observations(
    text: &str,
    source: &str,
    mut reported: Vec<Observation>,
) -> Result<Vec<Observation>> {
    let compact = |text: &str| {
        text.chars()
            .filter(|c| !c.is_whitespace())
            .flat_map(char::to_lowercase)
            .collect::<String>()
    };
    let year = Regex::new(r"\b20\d{2}\b")?;
    let date =
        Regex::new(r"(?i)(mar(?:ch)?|jun(?:e)?|sep(?:tember)?|dec(?:ember)?)[.,]?\s*(31|30)")?;
    let gap = Regex::new(r"\s{2,}")?;
    let mut blocks = Vec::new();
    for (page, text) in text.split('\u{c}').enumerate() {
        let title = if compact(text).contains("reconciliationsofnetearningstoffo") {
            "Reconciliations of Net Earnings to FFO"
        } else if compact(text).contains("consolidatedstatementsofincome") {
            "Consolidated Statements of Income"
        } else if compact(text).contains("calculationofpershareamounts") {
            "Calculation of Per Share Amounts"
        } else {
            continue;
        };
        let mut lines: Vec<String> = text.lines().map(str::to_owned).collect();
        if title == "Calculation of Per Share Amounts" {
            let Some(line) = lines.iter().find(|line| line.contains(title)) else {
                continue;
            };
            let start = line.find(title).unwrap();
            let right = Regex::new(r"\s{8,}\S")?
                .find(&line[start + title.len()..])
                .map(|matched| start + title.len() + matched.end() - 1);
            lines = lines
                .iter()
                .map(|line| {
                    line.chars()
                        .skip(start)
                        .take(right.map_or(usize::MAX, |end| end - start))
                        .collect()
                })
                .collect();
        }
        let Some(header) = lines
            .iter()
            .position(|line| compact(line).contains("threemonthsended"))
        else {
            continue;
        };
        let Some(year_row) = (header..lines.len().min(header + 6))
            .find(|index| matches!(year.find_iter(&lines[*index]).count(), 2 | 4))
        else {
            continue;
        };
        let years: Vec<_> = year
            .find_iter(&lines[year_row])
            .map(|matched| matched.as_str())
            .collect();
        let header_text = lines[header..=year_row].join(" ");
        let Some(date) = date.captures(&header_text) else {
            continue;
        };
        let month = match &date[1].to_lowercase()[..3] {
            "mar" => "March",
            "jun" => "June",
            "sep" => "September",
            _ => "December",
        };
        let duration = if compact(&header_text).contains("sixmonths") {
            "Six"
        } else if compact(&header_text).contains("ninemonths") {
            "Nine"
        } else {
            "Twelve"
        };
        let mut block = format!(
            "{title} Three Months Ended {month} {} {} in thousands {} ",
            &date[2],
            if years.len() == 4 {
                format!("{duration} Months Ended {month} {}", &date[2])
            } else {
                String::new()
            },
            years.join(" ")
        );
        for line in lines.iter().skip(year_row + 1) {
            let cells: Vec<_> = gap.split(line.trim()).collect();
            if cells.len() < 2 {
                continue;
            }
            let label = compact(cells[0]);
            let label = label.trim_end_matches('*');
            let label = match label {
                "affo" | "affoattributabletocommonstockholders/unitholders" => "AFFO",
                "nareitdefinedffoattributabletocommonstockholders/unitholders"
                | "subtotal-nareitdefinedffo" => {
                    "NAREIT defined FFO attributable to common stockholders/unitholders"
                }
                "weightedaveragecommonsharesoutstanding-diluted"
                | "weighted-averagecommonsharesoutstanding-diluted" => {
                    "Weighted average common shares outstanding - Diluted"
                }
                "affopershare-diluted" => "AFFO per share - Diluted",
                _ => continue,
            };
            block.push_str(&format!(
                "{label} {} {} END ",
                if label.starts_with("Weighted") {
                    ""
                } else {
                    "$"
                },
                cells[1..].join(" ")
            ));
        }
        blocks.push((format!("PDF page {}", page + 1), block));
    }
    let extracted = extract_blocks(blocks, source)?;
    reported.extend(extracted);
    Ok(reported)
}

fn extract_blocks(blocks: Vec<(String, String)>, source: &str) -> Result<Vec<Observation>> {
    let date = Regex::new(r"(?i)(mar(?:ch)?|jun(?:e)?|sep(?:tember)?|dec(?:ember)?)\.?\s+(31|30)")?;
    let years = Regex::new(r"\b((?:20\d{2}\s+){1,3}20\d{2})\b")?;
    let values = Regex::new(r"^\s*\$?\s*(\(?[+-]?\d[\d,]*(?:\.\d+)?\)?)")?;
    let labels = [
        (
            Metric::Ffo,
            Unit::Total,
            Regex::new(
                r"(?i)(?:NAREIT defined FFO attributable to common stockholders/unitholders|Subtotal\s*[-–—]\s*NAREIT defined FFO)\*?\s+\$",
            )?,
        ),
        (
            Metric::Affo,
            Unit::Total,
            Regex::new(r"(?i)\bAFFO(?: attributable to common stockholders/unitholders)?\*?\s+\$")?,
        ),
        (
            Metric::Affo,
            Unit::PerShare,
            Regex::new(r"(?i)\bAFFO per (?:common )?share\s*[-–—]\s*Diluted\*?\s+\$")?,
        ),
    ];
    let denominator =
        Regex::new(r"(?i)Weighted[- ]average common shares outstanding\s*[-–—]\s*Diluted\s+")?;
    let heading = Regex::new(r"(?i)Three Months Ended")?;
    let mut result = Vec::new();
    let mut shares = Vec::new();
    for (location, text) in blocks {
        let reconciliation = text.starts_with("Reconciliations of Net Earnings to FFO");
        let income = text.starts_with("Consolidated Statements of Income");
        let per_share = text.contains("Calculation of Per Share Amounts");
        if !reconciliation && !income && !per_share {
            continue;
        }
        let starts: Vec<_> = heading
            .find_iter(&text)
            .map(|matched| matched.start())
            .collect();
        for (index, start) in starts.iter().enumerate() {
            let section = &text[*start..starts.get(index + 1).copied().unwrap_or(text.len())];
            let Some(year_match) = years.captures(section) else {
                continue;
            };
            let header = &section[..year_match.get(0).unwrap().start()];
            let Some(date_match) = date.captures(header) else {
                continue;
            };
            let month = match &date_match[1].to_lowercase()[..3] {
                "mar" => 3,
                "jun" => 6,
                "sep" => 9,
                "dec" => 12,
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
            } else if header.contains("Twelve Months") || header.contains("Year Ended") {
                12
            } else {
                0
            };
            if years.len() == 4 && second_duration == 0 {
                continue;
            }
            let body = &section[year_match.get(0).unwrap().end()..];
            for (metric, unit, pattern) in &labels {
                if *unit == Unit::Total
                    && (!reconciliation
                        || !section[..year_match.get(0).unwrap().end()].contains("in thousands"))
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
                            published_on: None,
                            metric: *metric,
                            start: NaiveDate::MIN,
                            end: NaiveDate::MIN,
                            label: found.as_str().trim_end_matches(['*', '$', ' ']).to_owned(),
                            value: 0.0,
                            unit: *unit,
                            scale: if *unit == Unit::Total { 1000 } else { 1 },
                            source: source.to_owned(),
                            location: location.clone(),
                        },
                    );
                }
            }
            if income && section[..year_match.get(0).unwrap().end()].contains("in thousands") {
                for found in denominator.find_iter(body) {
                    append(
                        &mut shares,
                        &values,
                        &body[found.end()..],
                        &years,
                        (month, day, second_duration),
                        Observation {
                            published_on: None,
                            metric: Metric::Affo,
                            start: NaiveDate::MIN,
                            end: NaiveDate::MIN,
                            label: found.as_str().to_owned(),
                            value: 0.0,
                            unit: Unit::Total,
                            scale: 1000,
                            source: source.to_owned(),
                            location: location.clone(),
                        },
                    );
                }
            }
        }
    }
    let estimates: Vec<_> = result
        .iter()
        .filter(|total| total.metric == Metric::Affo && total.unit == Unit::Total)
        .filter_map(|total| {
            if result.iter().any(|value| {
                value.metric == Metric::Affo
                    && value.unit == Unit::PerShare
                    && value.start == total.start
                    && value.end == total.end
            }) {
                return None;
            }
            let matching: Vec<_> = shares
                .iter()
                .filter(|value| value.start == total.start && value.end == total.end)
                .collect();
            let shares = matching.first()?;
            if shares.value <= 0.0 || matching.iter().any(|value| value.value != shares.value) {
                return None;
            }
            Some(Observation {
                value: total.value / shares.value,
                unit: Unit::PerShare,
                scale: 1,
                label: "Calculated AFFO per diluted share/unit".into(),
                ..total.clone()
            })
        })
        .collect();
    result.extend(estimates);
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
        let Some(number) = crate::number(&value[1]) else {
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
) -> Result<Vec<(chrono::NaiveDate, String, Option<chrono::NaiveDate>)>> {
    anyhow::ensure!(
        company.symbol == "PLD",
        "no corporate archive adapter for {}",
        company.symbol
    );
    let archive = std::fs::read_to_string(
        sources
            .download_inner(
                company,
                "https://ir.prologis.com/financials/quarterly-results",
                true,
            )
            .await?,
    )?;
    let entries = {
        let html = scraper::Html::parse_document(&archive);
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
                let release_label = format!("Earnings Release Q{} {} HTML", &label[1], &label[2]);
                let releases: Vec<_> = html
                    .select(&scraper::Selector::parse("a[aria-label][href]").unwrap())
                    .filter(|link| link.value().attr("aria-label") == Some(release_label.as_str()))
                    .filter_map(|link| link.value().attr("href"))
                    .collect();
                anyhow::ensure!(
                    releases.len() == 1,
                    "PLD: missing or ambiguous earnings release for {end}"
                );
                result.push((end, url.to_owned(), releases[0].to_owned()));
            }
        }
        result
    };
    let mut result = Vec::new();
    for (end, url, release) in entries {
        let release = scraper::Html::parse_document(&sources.get(company, &release).await?);
        let dates: std::collections::BTreeSet<_> = release
            .select(&scraper::Selector::parse("time.date[datetime]").unwrap())
            .filter_map(|time| {
                time.value()
                    .attr("datetime")
                    .and_then(|date| date.get(..10))
            })
            .map(crate::publication::parse)
            .collect::<Result<_>>()?;
        anyhow::ensure!(
            dates.len() == 1,
            "PLD: missing or ambiguous supplemental publication date for {end}"
        );
        result.push((end, url, dates.into_iter().next()));
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
        "affo" | "affoattributabletocommonstockholders/unitholders"
            if pdf_page(page) && page.contains("inthousands") =>
        {
            Some((Metric::Affo, Unit::Total))
        }
        _ => None,
    }
}

fn debt(html: &scraper::Html, as_of: chrono::NaiveDate) -> anyhow::Result<crate::debt::Debt> {
    crate::debt::tables(html, as_of, &[crate::debt::Rule::new(4, 1000)], None)
}
