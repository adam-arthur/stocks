use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "SPG",
        1063761,
        &["www.simon.com", "investors.simon.com", "ir.simon.com"],
        vec![(
            Metric::Ffo,
            Unit::PerShare,
            r"^(?:basic and )?diluted ffo per share$",
            1,
        )],
        None,
    ))?;
    company.tables = crate::table::joined_parentheses;
    Ok(company)
}
