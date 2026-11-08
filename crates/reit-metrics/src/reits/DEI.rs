use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "DEI",
        1364250,
        &[
            "douglasemmett.com",
            "www.douglasemmett.com",
            "ir.douglasemmett.com",
        ],
        vec![(
            Metric::Ffo,
            Unit::PerShare,
            r"^ffo per (?:fully diluted share|share\s*[-–—]\s*fully diluted)$",
            1,
        )],
        None,
    ))?;
    company.tables = crate::table::joined_parentheses;
    Ok(company)
}
