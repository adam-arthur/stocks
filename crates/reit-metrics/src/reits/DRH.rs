use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "DRH",
        1298946,
        &["drhc.com", "www.drhc.com", "investor.drhc.com"],
        vec![
            (
                Metric::Ffo,
                Unit::Total,
                r"^ffo available to common stock(?: and unit)?\s*holders$",
                1000,
            ),
            (Metric::Ffo, Unit::Total, r"^ffo$", 1000),
        ],
        None,
    ))?;
    company.tables = crate::table::joined_parentheses;
    Ok(company)
}
