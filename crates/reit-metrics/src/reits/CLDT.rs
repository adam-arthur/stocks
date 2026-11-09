use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "CLDT",
        1476045,
        &["www.chathamlodgingtrust.com", "chathamlodgingtrust.com"],
        vec![(
            Metric::Ffo,
            Unit::Total,
            r"^ffo attribut(?:able|ed) to common (?:shareholders|share and unit holders)$",
            1000,
        )],
        None,
    ))?;
    company.tables = tables;
    company.debt = Some(debt);
    company.ownership = Some(crate::ownership::Rules::default());
    Ok(company)
}

fn tables(html: &str) -> Vec<crate::table::Table> {
    crate::table::joined_parentheses(html)
        .into_iter()
        .filter(|table| {
            !table
                .rows
                .iter()
                .take(6)
                .flatten()
                .any(|cell| matches!(cell.text.to_lowercase().as_str(), "low-end" | "high-end"))
        })
        .collect()
}

fn debt(html: &scraper::Html, as_of: chrono::NaiveDate) -> anyhow::Result<crate::debt::Debt> {
    crate::debt::tables(html, as_of, &[crate::debt::Rule::new(0, 1000)], None)
}
