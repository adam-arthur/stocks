use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "FPI",
        1591670,
        &[],
        vec![(Metric::Ffo, Unit::Total, r"^ffo$", 1000)],
        None,
    ))?;
    company.tables = crate::table::joined_parentheses;
    company.derive = Some(|documents| crate::derive::fourth_quarters("FPI", documents));
    Ok(company)
}
