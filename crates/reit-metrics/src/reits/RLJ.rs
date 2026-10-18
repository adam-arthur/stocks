use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "RLJ",
        1511337,
        &[
            "www.rljlodgingtrust.com",
            "rljlodgingtrust.com",
            "investor.rljlodgingtrust.com",
        ],
        vec![(Metric::Ffo, Unit::Total, r"^ffo$", 1000)],
        None,
    ))?;
    company.tables = crate::table::joined_parentheses;
    Ok(company)
}
