use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "SQFT",
        1080657,
        &["presidiopt.com", "www.presidiopt.com", "ir.presidiopt.com"],
        vec![(Metric::Ffo, Unit::Total, r"^ffo$", 1)],
        None,
    ))?;
    company.filings = &["8-K", "8-K/A", "10-Q", "10-Q/A", "10-K", "10-K/A"];
    company.tables = crate::table::joined_parentheses;
    company.debt = Some(debt);
    Ok(company)
}

fn debt(html: &scraper::Html, as_of: chrono::NaiveDate) -> anyhow::Result<crate::debt::Debt> {
    crate::debt::tables(html, as_of, &[crate::debt::Rule::new(2, 1)], None)
}
