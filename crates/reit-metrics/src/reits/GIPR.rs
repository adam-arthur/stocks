use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "GIPR",
        1651721,
        &[],
        vec![(
            Metric::Ffo,
            Unit::Total,
            r"^(?:ffo|funds from operations)(?: attributable to common stockholders)?$",
            1,
        )],
        None,
    ))?;
    company.filings = &["8-K", "8-K/A", "10-Q", "10-K"];
    company.derive = Some(|documents| crate::derive::fourth_quarters("GIPR", documents));
    company.tables = crate::table::joined_parentheses;
    Ok(company)
}
