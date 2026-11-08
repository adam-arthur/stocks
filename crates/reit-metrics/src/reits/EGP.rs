use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "EGP",
        49600,
        &["eastgroup.net", "www.eastgroup.net", "ir.eastgroup.net"],
        vec![(
            Metric::Ffo,
            Unit::PerShare,
            r"^(?:ffo|funds from operations) attributable to common stockholders$",
            1,
        )],
        None,
    ))?;
    company.tables = crate::table::joined_parentheses;
    company.rules[0].section = Some(regex::Regex::new(r"(?i)^diluted per common share data")?);
    Ok(company)
}
