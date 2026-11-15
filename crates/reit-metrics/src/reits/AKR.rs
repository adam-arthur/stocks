use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "AKR",
        899629,
        &["ir.acadiarealty.com", "www.acadiarealty.com"],
        vec![
            (Metric::Affo, Unit::PerShare, r"^affo$", 1),
            (
                Metric::Affo,
                Unit::Total,
                r"^affo to common shareholders and common op unit holders$",
                1000,
            ),
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^(?:diluted )?funds from operations, per common share and common op unit$",
                1,
            ),
            (
                Metric::Ffo,
                Unit::Total,
                r"^funds from operations attributable to common shareholders and common op unit holders(?: - diluted)?$",
                1000,
            ),
        ],
        None,
    ))?;
    company.rules[0].section = Some(regex::Regex::new(
        r"(?i)^diluted ffo per common share and op unit:$",
    )?);
    company.tables = tables;
    company.debt = Some(debt);
    company.sections.push(affo_per_share);
    company.affo_support = crate::AffoSupport::Reported;
    Ok(company)
}

fn tables(html: &str) -> Vec<crate::table::Table> {
    let mut tables = crate::table::tables(html);
    for table in &mut tables {
        if table
            .rows
            .iter()
            .flatten()
            .any(|cell| cell.text.to_lowercase().starts_with("march 31"))
            && !table.rows.iter().flatten().any(|cell| {
                ["june", "september", "december"]
                    .iter()
                    .any(|month| cell.text.to_lowercase().contains(month))
            })
        {
            for cell in table.rows.iter_mut().flatten() {
                if cell.text.eq_ignore_ascii_case("year-to-date") {
                    cell.text = "three months ended".into();
                }
            }
        }
    }
    for cell in tables
        .iter_mut()
        .flat_map(|table| table.rows.iter_mut().flatten())
    {
        cell.text = match cell.text.to_lowercase().as_str() {
            "3 months ended" => "three months ended".into(),
            "6 months ended" => "six months ended".into(),
            "9 months ended" => "nine months ended".into(),
            "12 months ended" => "twelve months ended".into(),
            _ => continue,
        };
    }
    tables
}

fn debt(html: &scraper::Html, as_of: chrono::NaiveDate) -> anyhow::Result<crate::debt::Debt> {
    crate::debt::tables(html, as_of, &[crate::debt::Rule::new(0, 1000)], None)
}

fn affo_per_share(tables: &[crate::table::Table], source: &str) -> Result<Vec<crate::Observation>> {
    let year = regex::Regex::new(r"\b(20\d{2})\b")?;
    let date = regex::Regex::new(r"(?i)(march|june|september|december)\s*(31|30)")?;
    let quarter = regex::Regex::new(r"(?i)^Q([1-4])(?:\s+(20\d{2}))?$")?;
    let range = regex::Regex::new(r"a^")?;
    let shares =
        regex::Regex::new(r"(?i)^total weighted[- ]average diluted shares and op units:?$")?;
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
                    == Some("affo to common shareholders and common op unit holders")
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
