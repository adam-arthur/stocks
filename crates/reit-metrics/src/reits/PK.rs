use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "PK",
        1617406,
        &[
            "pkhotelsandresorts.com",
            "www.pkhotelsandresorts.com",
            "investors.pkhotelsandresorts.com",
        ],
        vec![(
            Metric::Ffo,
            Unit::PerShare,
            r"^(?:nareit )?ffo per share\s*[-–—]\s*diluted$",
            1,
        )],
        None,
    ))?;
    company.tables = crate::table::joined_parentheses;
    Ok(company)
}
