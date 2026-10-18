use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "SKT",
        899715,
        &[
            "www.tanger.inc",
            "investors.tanger.inc",
            "investors.tangeroutlet.com",
        ],
        vec![(
            Metric::Ffo,
            Unit::PerShare,
            r"^ffo (?:available to common shareholders per share\s*[-–—]\s*diluted|per share)$",
            1,
        )],
        None,
    ))?;
    company.tables = crate::table::joined_parentheses;
    Ok(company)
}
