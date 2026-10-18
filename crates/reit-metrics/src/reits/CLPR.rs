use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "CLPR",
        1649096,
        &["www.clipperrealty.com", "investors.clipperrealty.com"],
        vec![(Metric::Ffo, Unit::Total, r"^ffo$", 1000)],
        None,
    ))?;
    company.tables = crate::table::joined_parentheses;
    Ok(company)
}
