use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "DHC",
        1075415,
        &["dhcreit.com", "www.dhcreit.com", "snhr.com", "www.snhr.com"],
        vec![
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^ffo per (?:common )?share(?: \(basic and diluted\))?$",
                1,
            ),
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^ffo(?: (?:available for|attributable to) common shareholders)?$",
                1,
            ),
        ],
        None,
    ))?;
    company.tables = crate::table::joined_parentheses;
    company.rules[1].section = Some(regex::Regex::new(
        r"(?i)^(?:per (?:common )?share (?:data|amounts)|basic and diluted per common share amounts)",
    )?);
    company.extra_observations = Some(extract);
    company.debt = Some(debt);
    Ok(company)
}

fn extract(html: &str, source: &str) -> Result<Vec<crate::Observation>> {
    use chrono::{Datelike, NaiveDate};

    let dates = regex::Regex::new(r"\b(3|6|9|12)/(30|31)/(20\d{2})\b")?;
    let section = regex::Regex::new(r"(?i)per (?:common )?share data\s*\(basic and diluted\):")?;
    let mut observations = Vec::new();
    for (page, text) in crate::presentation::pages(html)?.iter().enumerate() {
        let Some(section) = section.find(text) else {
            continue;
        };
        let headers = dates
            .captures_iter(&text[..section.start()])
            .filter_map(|date| NaiveDate::parse_from_str(&date[0], "%m/%d/%Y").ok())
            .take(5)
            .collect::<Vec<_>>();
        if !text.to_lowercase().contains("for the three months ended") || headers.len() != 5 {
            continue;
        }
        let Some(values) = crate::presentation::values(&text[section.end()..], "FFO")?
            .into_iter()
            .next()
            .filter(|values| values.len() == headers.len())
        else {
            continue;
        };
        for (column, (end, value)) in headers.into_iter().zip(values).enumerate() {
            observations.push(crate::Observation {
                published_on: None,
                metric: Metric::Ffo,
                start: NaiveDate::from_ymd_opt(end.year(), end.month() - 2, 1).unwrap(),
                end,
                label: "FFO per common share (basic and diluted)".into(),
                value,
                unit: Unit::PerShare,
                scale: 1,
                source: source.to_owned(),
                location: format!("presentation page {}, column {}", page + 1, column + 1),
            });
        }
    }
    Ok(observations)
}

fn debt(html: &scraper::Html, as_of: chrono::NaiveDate) -> anyhow::Result<crate::debt::Debt> {
    crate::debt::tables(html, as_of, &[crate::debt::Rule::new(0, 1000)], None)
}
