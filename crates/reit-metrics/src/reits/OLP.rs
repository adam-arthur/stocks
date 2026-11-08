use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "OLP",
        712770,
        &["1liberty.com", "www.1liberty.com"],
        vec![(
            Metric::Ffo,
            Unit::PerShare,
            r"^(?:nareit funds from operations per share of common stock\s*[-–—]\s*diluted|funds from operations per common share\s*[-–—]\s*diluted\s*[-–—]\s*note 2)$",
            1,
        )],
        None,
    ))?;
    company.tables = crate::table::joined_parentheses;
    Ok(company)
}
