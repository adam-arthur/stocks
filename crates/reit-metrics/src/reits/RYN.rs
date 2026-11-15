use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "RYN",
        52827,
        &["www.rayonier.com", "rayonier.com", "ir.rayonier.com"],
        vec![(
            Metric::Affo,
            Unit::Total,
            r"^cash available for distribution$",
            1000000,
        )],
        None,
    ))?;
    company.affo_support = crate::AffoSupport::Reported;
    company.sections.push(affo_per_share);
    company.tables = crate::table::joined_parentheses;
    company.debt = Some(debt);
    Ok(company)
}

fn debt(html: &scraper::Html, as_of: chrono::NaiveDate) -> anyhow::Result<crate::debt::Debt> {
    crate::debt::tables(html, as_of, &[crate::debt::Rule::new(0, 1000)], None)
}

fn affo_per_share(tables: &[crate::table::Table], source: &str) -> Result<Vec<crate::Observation>> {
    crate::affo_per_share(
        company()?,
        &tables
            .iter()
            .filter(|table| {
                !table.rows.iter().flatten().any(|cell| {
                    cell.text
                        .to_lowercase()
                        .starts_with("cash available for distribution")
                }) || table.rows.iter().flatten().any(|cell| {
                    cell.text.to_lowercase()
                        == "cash provided by operating activities from discontinued operations"
                })
            })
            .map(|table| crate::table::Table {
                rows: table.rows.clone(),
                amount_scale: table.amount_scale,
            })
            .collect::<Vec<_>>(),
        source,
        crate::companies::Rule {
            metric: Metric::Affo,
            unit: Unit::Total,
            label: regex::Regex::new(r"^diluted eps$")?,
            scale: Some(1),
            section: None,
        },
    )
}
