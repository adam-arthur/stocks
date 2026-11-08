use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "DEA",
        1622194,
        &[],
        vec![
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^ffo,? per share\s*[-–—]\s*(?:weighted average )?fully diluted basis$",
                1,
            ),
            (
                Metric::Ffo,
                Unit::Total,
                r"^(?:funds from operations|ffo)$",
                1000,
            ),
        ],
        None,
    ))?;
    company.tables = crate::table::joined_parentheses;
    Ok(company)
}
