use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "FSP",
        1031316,
        &[],
        vec![(Metric::Ffo, Unit::Total, r"^nareit ffo$", 1000)],
        None,
    ))?;
    company.tables = crate::table::joined_parentheses;
    Ok(company)
}
