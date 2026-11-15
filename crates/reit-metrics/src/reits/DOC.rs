use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "DOC",
        765880,
        &["healthpeak.com", "www.healthpeak.com", "ir.healthpeak.com"],
        vec![
            (
                Metric::Affo,
                Unit::PerShare,
                r"^diluted affo per common share$",
                1,
            ),
            (
                Metric::Affo,
                Unit::Total,
                r"^diluted (?:affo|fad) applicable to common shares$",
                1000,
            ),
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^diluted (?:nareit )?ffo per common share$",
                1,
            ),
        ],
        None,
    ))?;
    company.tables = tables;
    company.sections.push(affo_per_share);
    company.affo_support = crate::AffoSupport::Reported;
    company.debt = Some(debt);
    Ok(company)
}

fn debt(html: &scraper::Html, as_of: chrono::NaiveDate) -> anyhow::Result<crate::debt::Debt> {
    crate::debt::tables(html, as_of, &[crate::debt::Rule::new(7, 1000)], None)
}

fn affo_per_share(tables: &[crate::table::Table], source: &str) -> Result<Vec<crate::Observation>> {
    crate::affo_per_share(
        company()?,
        tables,
        source,
        crate::companies::Rule {
            metric: Metric::Affo,
            unit: Unit::Total,
            label: regex::Regex::new(
                r"^weighted average shares outstanding - diluted (?:affo|fad)$",
            )?,
            scale: Some(1000),
            section: None,
        },
    )
}

fn tables(html: &str) -> Vec<crate::table::Table> {
    crate::table::joined_parentheses(html)
        .into_iter()
        .map(|mut table| {
            if !table.rows.iter().flatten().any(|cell| {
                cell.text
                    .to_lowercase()
                    .starts_with("weighted average shares")
            }) {
                table.rows.retain(|row| {
                    !row.first().is_some_and(|cell| {
                        cell.text
                            .to_lowercase()
                            .starts_with("diluted affo applicable to common shares")
                    })
                });
            }
            table
        })
        .collect()
}
