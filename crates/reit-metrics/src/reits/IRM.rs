use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "IRM",
        1020569,
        &[
            "www.ironmountain.com",
            "investors.ironmountain.com",
            "ir.ironmountain.com",
        ],
        vec![
            (Metric::Ffo, Unit::PerShare, r"^nareit ffo$", 1),
            (Metric::Affo, Unit::Total, r"^affo$", 1000),
            (Metric::Affo, Unit::PerShare, r"^affo per share$", 1),
        ],
        None,
    ))?;
    company.rules[0].section = Some(regex::Regex::new(r"(?i)^per share amounts")?);
    company.affo_support = crate::AffoSupport::Reported;
    company.tables = tables;
    company.extra_observations = Some(extract);
    company.sections.push(affo_per_share);
    company.debt = Some(debt);
    Ok(company)
}

pub(crate) fn tables(html: &str) -> Vec<crate::table::Table> {
    let mut tables = crate::table::joined_parentheses(html);
    let quarter = regex::Regex::new(r"^Q([1-4]) 20\d{2}$").unwrap();
    for table in &mut tables {
        if table
            .rows
            .iter()
            .take(6)
            .flatten()
            .any(|cell| cell.text.to_lowercase().contains("in millions"))
        {
            table.rows.retain(|row| {
                !row.iter()
                    .any(|cell| cell.text.eq_ignore_ascii_case("AFFO"))
            });
        }
        let month = table.rows.iter().take(6).flatten().find_map(|cell| {
            quarter
                .captures(&cell.text)
                .map(|date| date[1].parse::<usize>().unwrap())
        });
        for cell in table.rows.iter_mut().flatten() {
            if cell.text.eq_ignore_ascii_case("FFO (Nareit)") {
                cell.text = "Nareit FFO".into();
            }
            if let Some(quarter) = month
                && let Some(year) = cell.text.strip_prefix("YTD ")
                && year.parse::<i32>().is_ok()
            {
                cell.text = format!(
                    "{} Months Ended {} {}",
                    ["Three", "Six", "Nine", "Twelve"][quarter - 1],
                    ["March 31", "June 30", "September 30", "December 31"][quarter - 1],
                    year
                );
            }
        }
    }
    tables
}

pub(crate) fn extract(html: &str, source: &str) -> Result<Vec<crate::Observation>> {
    use chrono::{Datelike, NaiveDate};

    let quarter = regex::Regex::new(r"\bQ([1-4])\s+(20\d{2})\b")?;
    let pair = regex::Regex::new(r"\bQ([1-4])\s+(20\d{2})\s+Q([1-4])\s+(20\d{2})\b")?;
    let ytd = regex::Regex::new(r"\bYTD\s+(20\d{2})\b")?;
    let pages = crate::presentation::pages(html)?;
    let report = pages
        .iter()
        .filter(|text| {
            text.to_lowercase().contains("reconciliation of net")
                && text.to_lowercase().contains("ffo")
        })
        .flat_map(|text| quarter.captures_iter(text))
        .filter_map(|date| {
            NaiveDate::from_ymd_opt(date[2].parse().ok()?, date[1].parse::<u32>().ok()? * 3, 1)
        })
        .max();
    let footnotes = regex::Regex::new(r"%\s+\([1-9]\)\s+[A-Za-z]")?;
    let mut observations = Vec::new();
    for (page, text) in pages.iter().enumerate() {
        if !text.to_lowercase().contains("reconciliation of net") {
            continue;
        }
        let rows = footnotes
            .find(text)
            .map_or(text.as_str(), |note| &text[..note.start() + 1]);
        for (metric, label, affo_unit) in [
            (Metric::Ffo, r"FFO\s*\(NAREIT\)", None),
            (Metric::Affo, r"AFFO", Some(Unit::Total)),
            (Metric::Affo, r"AFFO\s+Per\s+Share", Some(Unit::PerShare)),
        ] {
            let Some(values) = crate::presentation::values(rows, label)?.into_iter().last() else {
                continue;
            };
            let periods = if values.len() == 4 || (values.len() == 2 && pair.is_match(text)) {
                let Some(header) = pair.captures_iter(text).last() else {
                    continue;
                };
                if header[1] != header[3] {
                    continue;
                }
                let mut years = [header[2].parse::<i32>()?, header[4].parse::<i32>()?];
                if text
                    .to_lowercase()
                    .ends_with("reconciliation of net income to ffo & affo")
                {
                    years.reverse();
                }
                (0..values.len())
                    .map(|column| {
                        Ok((years[column % 2], header[1].parse::<u32>()? * 3, column < 2))
                    })
                    .collect::<Result<Vec<_>>>()?
            } else if values.len() == 3 {
                quarter
                    .captures_iter(text)
                    .take(3)
                    .map(|date| Ok((date[2].parse::<i32>()?, date[1].parse::<u32>()? * 3, true)))
                    .collect::<Result<Vec<_>>>()?
            } else if values.len() == 2 {
                let Some(report) = report else {
                    continue;
                };
                ytd.captures_iter(text)
                    .take(2)
                    .map(|date| Ok((date[1].parse::<i32>()?, report.month(), false)))
                    .collect::<Result<Vec<_>>>()?
            } else {
                continue;
            };
            if periods.len() != values.len() {
                continue;
            }
            let unit = affo_unit.unwrap_or(
                if text
                    .chars()
                    .filter(|character| !character.is_whitespace())
                    .collect::<String>()
                    .to_lowercase()
                    .contains("fullydilutedshares")
                {
                    Unit::PerShare
                } else {
                    Unit::Total
                },
            );
            for ((year, month, quarterly), value) in periods.into_iter().zip(values) {
                let end = NaiveDate::from_ymd_opt(year, month, 1)
                    .unwrap()
                    .checked_add_months(chrono::Months::new(1))
                    .unwrap()
                    .pred_opt()
                    .unwrap();
                observations.push(crate::Observation {
                    published_on: None,
                    metric,
                    start: NaiveDate::from_ymd_opt(year, if quarterly { month - 2 } else { 1 }, 1)
                        .unwrap(),
                    end,
                    label: if metric == Metric::Ffo {
                        "FFO (Nareit)"
                    } else {
                        "AFFO"
                    }
                    .into(),
                    value,
                    unit,
                    scale: if unit == Unit::Total { 1000 } else { 1 },
                    source: source.to_owned(),
                    location: format!("presentation page {}", page + 1),
                });
            }
        }
    }
    Ok(observations)
}

fn affo_per_share(tables: &[crate::table::Table], source: &str) -> Result<Vec<crate::Observation>> {
    crate::affo_per_share(
        company()?,
        tables,
        source,
        crate::companies::Rule {
            metric: Metric::Affo,
            unit: Unit::Total,
            label: regex::Regex::new(r"^weighted average common shares outstanding - diluted$")?,
            scale: Some(1000),
            section: None,
        },
    )
}

fn debt(html: &scraper::Html, as_of: chrono::NaiveDate) -> anyhow::Result<crate::debt::Debt> {
    crate::debt::tables(html, as_of, &[crate::debt::Rule::new(0, 1000)], None)
}
