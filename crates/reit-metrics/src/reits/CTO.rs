use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "CTO",
        23795,
        &["www.ctoreit.com", "ctoreit.com", "ir.ctoreit.com"],
        vec![
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^ffo(?: attributable to common stockholders)? per (?:common share [-–] diluted|diluted share)$",
                1,
            ),
            (
                Metric::Ffo,
                Unit::Total,
                r"^funds from operations attributable to common stockholders$",
                1000,
            ),
        ],
        None,
    ))?;
    company.tables = crate::table::joined_parentheses;
    Ok(company)
}
