use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "INN",
        1497645,
        &["www.shpreit.com", "investor.shpreit.com"],
        vec![
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^ffo per diluted share and unit(?: \([\d, ]+\))?$",
                1,
            ),
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^ffo per (?:common share\s*/\s*common unit|share of common stock and common units)$",
                1,
            ),
            (
                Metric::Ffo,
                Unit::Total,
                r"^ffo applicable to (?:common shares and common units|common stock and common units)$",
                1000,
            ),
        ],
        None,
    ))?;
    company.tables = crate::table::joined_parentheses;
    company.additional_items = Some(|items| matches!(items.trim(), "9.01" | "8.01,9.01"));
    company.rules.extend([
        crate::companies::Rule { metric: Metric::Affo, unit: Unit::PerShare, label: regex::Regex::new(r"(?i)^adjusted ffo per diluted share and unit$")?, scale: Some(1), section: None },
        crate::companies::Rule { metric: Metric::Affo, unit: Unit::PerShare, label: regex::Regex::new(r"(?i)^affo per (?:common share\s*/\s*common unit|(?:share of )?common stock and common units|common share and (?:common )?unit)$")?, scale: Some(1), section: None },
        crate::companies::Rule { metric: Metric::Affo, unit: Unit::Total, label: regex::Regex::new(r"(?i)^(?:adjusted (?:ffo|funds from operations)|affo applicable to (?:common shares and common units|common stock and common units))$")?, scale: Some(1000), section: None },
    ]);
    company.extra_observations = Some(earnings_summary);
    company.affo_comparatives = true;
    company.affo_support = crate::AffoSupport::Reported;
    company.debt = Some(debt);
    Ok(company)
}

fn debt(html: &scraper::Html, as_of: chrono::NaiveDate) -> anyhow::Result<crate::debt::Debt> {
    crate::debt::tables(html, as_of, &[crate::debt::Rule::new(0, 1000)], None)
}

fn earnings_summary(html: &str, source: &str) -> Result<Vec<crate::Observation>> {
    use chrono::{Datelike, NaiveDate};
    let heading = regex::Regex::new(
        r"(?i)results for the three.{0,35}?months.{0,30}?ended ([a-z]+ [0-9]+, 20[0-9]{2}),? are as follows .in thousands, except per share amounts",
    )?;
    let mut observations = Vec::new();
    for (page, text) in crate::presentation::pages(html)?.iter().enumerate() {
        let Some(header) = heading.captures(text) else {
            continue;
        };
        let end = crate::publication::parse(&header[1])?;
        let section = &text[header.get(0).unwrap().end()..];
        if !section
            .to_lowercase()
            .contains("for the three months ended")
        {
            continue;
        }
        for (metric, label, unit, scale) in [
            (Metric::Ffo, "FFO", Unit::Total, 1000),
            (
                Metric::Ffo,
                "FFO per diluted share and unit",
                Unit::PerShare,
                1,
            ),
            (Metric::Affo, "Adjusted FFO", Unit::Total, 1000),
            (
                Metric::Affo,
                "Adjusted FFO per diluted share and unit",
                Unit::PerShare,
                1,
            ),
        ] {
            let content = if metric == Metric::Ffo {
                section.replace("Adjusted FFO", "Adjusted metric")
            } else {
                section.to_owned()
            };
            if let Some(value) = crate::presentation::values(&content, label)?
                .first()
                .and_then(|r| r.first())
                .copied()
            {
                observations.push(crate::Observation {
                    published_on: None,
                    metric,
                    start: NaiveDate::from_ymd_opt(end.year(), end.month() - 2, 1).unwrap(),
                    end,
                    label: label.into(),
                    unit,
                    scale,
                    value,
                    source: source.into(),
                    location: format!("quarterly earnings summary, page {}", page + 1),
                });
            }
        }
    }
    Ok(observations)
}
