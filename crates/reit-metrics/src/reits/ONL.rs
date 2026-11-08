use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "ONL",
        1873923,
        &[
            "onlreit.com",
            "www.onlreit.com",
            "orionproperties.com",
            "www.orionproperties.com",
        ],
        vec![(
            Metric::Ffo,
            Unit::PerShare,
            r"^ffo attributable to common stockholders per (?:diluted )?share$",
            1,
        )],
        None,
    ))?;
    company.tables = crate::table::joined_parentheses;
    Ok(company)
}
