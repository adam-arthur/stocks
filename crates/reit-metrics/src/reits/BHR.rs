use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "BHR",
        1574085,
        &["www.bhrreit.com", "bhrreit.com"],
        vec![(
            Metric::Ffo,
            Unit::Total,
            r"^ffo available to common stockholders and op unitholders$",
            1000,
        )],
        None,
    ))?;
    company.tables = crate::table::joined_parentheses;
    Ok(company)
}
