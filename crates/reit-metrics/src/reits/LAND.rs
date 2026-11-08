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
                r"^(?:(?:diluted )?ffo per weighted[- ]average (?:common|total(?: common)?) share|ffo available to common stockholders(?: and (?:non-controlling op unitholders|op unit ?holders))?\s*[-–—]\s*diluted)$",
                1,
            ),
            (
                Metric::Ffo,
                Unit::Total,
                r"^ffo available to common stockholders(?: and (?:non-controlling op unitholders|op unit ?holders))?$",
                0,
            ),
        ],
        None,
    ))?;
    company.tables = crate::table::joined_parentheses;
    Ok(company)
}
