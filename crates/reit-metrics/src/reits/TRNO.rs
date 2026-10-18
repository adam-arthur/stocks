use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "TRNO",
        1476150,
        &["terreno.com", "www.terreno.com", "ir.terreno.com"],
        vec![(
            Metric::Ffo,
            Unit::PerShare,
            r"^(?:basic and )?diluted ffo per common share$",
            1,
        )],
        None,
    ))?;
    company.filings = &["10-Q", "10-Q/A", "10-K", "10-K/A"];
    company.tables = crate::table::joined_parentheses;
    Ok(company)
}
