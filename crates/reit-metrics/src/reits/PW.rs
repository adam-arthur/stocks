use crate::companies::{Company, build};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "PW",
        1532619,
        &["pwreit.com", "www.pwreit.com"],
        vec![],
        None,
    ))?;
    company.filings = &["8-K", "8-K/A", "10-Q", "10-Q/A", "10-K", "10-K/A"];
    company.tables = crate::table::joined_parentheses;
    Ok(company)
}
