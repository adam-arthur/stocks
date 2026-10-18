use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "LAND",
        1495240,
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
                r"^ffo available to common stockholders(?: and non-controlling op unitholders)?$",
                1000,
            ),
        ],
        None,
    ))?;
    company.tables = crate::table::joined_parentheses;
    Ok(company)
}
