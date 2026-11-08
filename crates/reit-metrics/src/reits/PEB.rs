use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "PEB",
        1474098,
        &[
            "pebblebrookhotels.com",
            "www.pebblebrookhotels.com",
            "investor.pebblebrookhotels.com",
        ],
        vec![(
            Metric::Ffo,
            Unit::PerShare,
            r"^ffo per common share\s*[-–—]\s*diluted$",
            1,
        )],
        None,
    ))?;
    company.tables = crate::table::joined_parentheses;
    Ok(company)
}
