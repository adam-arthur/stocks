use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "RHP",
        1040829,
        &["www.rymanhp.com", "rymanhp.com", "ir.rymanhp.com"],
        vec![(
            Metric::Ffo,
            Unit::PerShare,
            r"^ffo(?: available to common (?:shareholders|stockholders)(?: and unit holders)?)? per diluted share(?:/unit)?$",
            1,
        )],
        None,
    ))?;
    company.tables = crate::table::joined_parentheses;
    Ok(company)
}
