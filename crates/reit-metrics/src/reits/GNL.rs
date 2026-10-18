use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "GNL",
        1526113,
        &[],
        vec![
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^(?:nareit defined )?ffo per (?:diluted )?common share$",
                1,
            ),
            (
                Metric::Ffo,
                Unit::Total,
                r"^ffo \((?:as )?defined by nareit\)(?: attributable to (?:common )?stockholders)?(?: \[1\])?$",
                1000,
            ),
        ],
        None,
    ))?;
    company.tables = crate::table::joined_parentheses;
    Ok(company)
}
