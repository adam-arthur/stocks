use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "UMH",
        752642,
        &["umh.reit", "www.umh.reit", "ir.umh.reit"],
        vec![(
            Metric::Ffo,
            Unit::PerShare,
            r"^ffo (?:per share\s*[-–]\s*(?:basic and )?diluted|per diluted common share)$",
            1,
        )],
        None,
    ))?;
    company.tables = crate::table::joined_parentheses;
    Ok(company)
}
