use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "NHI",
        877860,
        &["investors.nhireit.com", "www.nhireit.com"],
        vec![
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^nareit ffo attributable to common stockholders per share$",
                1,
            ),
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^nareit ffo per diluted share$",
                1,
            ),
            (
                Metric::Ffo,
                Unit::Total,
                r"^nareit ffo(?: attributable to common stockholders)?$",
                1000,
            ),
        ],
        None,
    ))?;
    company.tables = crate::table::joined_parentheses;
    company.rules[0].section = Some(regex::Regex::new(r"(?i)^diluted:?")?);
    Ok(company)
}
