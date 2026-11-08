use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "DOC",
        765880,
        &["healthpeak.com", "www.healthpeak.com", "ir.healthpeak.com"],
        vec![(
            Metric::Ffo,
            Unit::PerShare,
            r"^diluted (?:nareit )?ffo per common share$",
            1,
        )],
        None,
    ))?;
    company.tables = crate::table::joined_parentheses;
    Ok(company)
}
