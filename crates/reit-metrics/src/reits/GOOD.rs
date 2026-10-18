use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "GOOD",
        1234006,
        &[],
        vec![
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^ffo available to common stockholders(?: and non-controlling op unitholders)?\s*[-–—]\s*diluted$",
                1,
            ),
            (
                Metric::Ffo,
                Unit::Total,
                r"^funds from operations available to common stockholders(?: and non-controlling op unitholders)?\s*[-–—]\s*diluted$",
                1000,
            ),
        ],
        Some("ffo available to common stockholders"),
    ))?;
    company.tables = crate::table::joined_parentheses;
    Ok(company)
}
