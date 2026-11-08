use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "PSTL",
        1759774,
        &[
            "postalrealtytrust.com",
            "www.postalrealtytrust.com",
            "investor.postalrealtytrust.com",
        ],
        vec![(
            Metric::Ffo,
            Unit::PerShare,
            r"^ffo per (?:share|common share and common unit outstanding(?:\s*[-–—]\s*diluted)?)$",
            1,
        )],
        None,
    ))?;
    company.tables = crate::table::joined_parentheses;
    Ok(company)
}
