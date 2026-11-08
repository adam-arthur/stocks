use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "MAC",
        912242,
        &["investing.macerich.com", "www.macerich.com"],
        vec![
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^ffo per share\s*[-–—]\s*(?:basic and )?diluted$",
                1,
            ),
            (
                Metric::Ffo,
                Unit::Total,
                r"^ffo(?: attributable to common stockholders and unit holders)?\s*[-–—]\s*basic and diluted$",
                1000,
            ),
        ],
        None,
    ))?;
    company.tables = crate::table::joined_parentheses;
    Ok(company)
}
