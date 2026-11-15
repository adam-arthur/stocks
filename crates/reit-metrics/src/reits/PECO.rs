use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "PECO",
        1476204,
        &[
            "phillipsedison.com",
            "www.phillipsedison.com",
            "investors.phillipsedison.com",
        ],
        vec![
            (
                Metric::Affo,
                Unit::Total,
                r"^adjusted ffo attributable to stockholders and op unit holders$",
                1000,
            ),
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^(?:nareit )?ffo(?: attributable to stockholders and (?:convertible noncontrolling interests|op unit holders))? per share\s*[-–—]\s*(?:basic and )?diluted$",
                1,
            ),
        ],
        None,
    ))?;
    company.filings = &["8-K", "8-K/A", "10-Q", "10-Q/A", "10-K", "10-K/A"];
    company.tables = crate::table::joined_parentheses;
    company.debt = Some(debt);
    company.sections.push(affo_per_share);
    company.affo_support = crate::AffoSupport::Reported;
    Ok(company)
}

fn debt(html: &scraper::Html, as_of: chrono::NaiveDate) -> anyhow::Result<crate::debt::Debt> {
    crate::debt::tables(
        html,
        as_of,
        &[crate::debt::Rule {
            row: Some(("Total", 0)),
            ..crate::debt::Rule::new(0, 1000)
        }],
        None,
    )
}

fn affo_per_share(tables: &[crate::table::Table], source: &str) -> Result<Vec<crate::Observation>> {
    let year = regex::Regex::new(r"\b(20\d{2})\b")?;
    let date = regex::Regex::new(r"(?i)(march|june|september|december)\s*(31|30)")?;
    let quarter = regex::Regex::new(r"(?i)^Q([1-4])(?:\s+(20\d{2}))?$")?;
    let range = regex::Regex::new(r"a^")?;
    let shares =
        regex::Regex::new(r"(?i)^weighted-average shares of common stock outstanding - diluted$")?;
    let mut observations = Vec::new();
    for (index, table) in tables
        .iter()
        .enumerate()
        .filter(|(_, table)| !table.is_forecast())
    {
        let label = |row: &Vec<crate::table::Cell>| {
            row.iter()
                .find(|cell| !cell.text.is_empty())
                .map(|cell| cell.text.to_lowercase())
        };
        let numerators: Vec<_> = table
            .rows
            .iter()
            .enumerate()
            .filter(|(_, row)| {
                label(row).as_deref()
                    == Some("adjusted ffo attributable to stockholders and op unit holders")
            })
            .collect();
        if numerators.len() != 1 {
            continue;
        }
        let row = numerators[0].0;
        if table
            .rows
            .iter()
            .any(|row| label(row).as_deref() == Some("affo"))
        {
            continue;
        }
        let denominators: Vec<_> = table
            .rows
            .iter()
            .filter(|row| label(row).is_some_and(|label| shares.is_match(&label)))
            .collect();
        if denominators.len() != 1 {
            continue;
        }
        let headers: Vec<_> = table.rows[..row]
            .iter()
            .filter(|row| {
                row.iter().any(|cell| {
                    year.is_match(&cell.text)
                        || date.is_match(&cell.text)
                        || quarter.is_match(&cell.text)
                        || crate::duration(&cell.text).is_some()
                }) && row
                    .iter()
                    .filter_map(|cell| crate::number(&cell.text))
                    .all(|value| (2000.0..2100.0).contains(&value))
            })
            .flatten()
            .cloned()
            .collect();
        for cell in table.rows[row].iter() {
            let Some(value) = crate::number(&cell.text) else {
                continue;
            };
            let Some((start, end)) =
                crate::period(&headers, cell.column, &year, &date, &quarter, &range)
            else {
                continue;
            };
            let matching: Vec<_> = denominators[0]
                .iter()
                .filter_map(|cell| {
                    let value = crate::number(&cell.text)?;
                    (crate::period(&headers, cell.column, &year, &date, &quarter, &range)
                        == Some((start, end)))
                    .then_some(value)
                })
                .collect();
            if matching.len() != 1 || matching[0] <= 0.0 {
                continue;
            }
            observations.push(crate::Observation {
                metric: Metric::Affo,
                published_on: None,
                start,
                end,
                label: "Calculated AFFO per diluted common share and OP unit".into(),
                value: value / matching[0],
                unit: Unit::PerShare,
                scale: 1,
                source: source.into(),
                location: format!("table {index}, row {row}"),
            });
        }
    }
    Ok(observations)
}
