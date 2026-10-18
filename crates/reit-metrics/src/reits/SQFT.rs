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
    Ok(company)
}
