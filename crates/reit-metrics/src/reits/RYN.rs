use crate::companies::{Company, build};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "RYN",
        52827,
        &["www.rayonier.com", "rayonier.com", "ir.rayonier.com"],
        vec![],
        None,
    ))?;
    company.tables = crate::table::joined_parentheses;
    company.debt = Some(debt);
    Ok(company)
}

fn debt(html: &scraper::Html, as_of: chrono::NaiveDate) -> anyhow::Result<crate::debt::Debt> {
    crate::debt::tables(html, as_of, &[crate::debt::Rule::new(0, 1000)], None)
}
