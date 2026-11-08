use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "NNN",
        751364,
        &["www.nnnreit.com", "nnnreit.com"],
        vec![
            (Metric::Ffo, Unit::PerShare, r"^ffo per share:? diluted$", 1),
            (Metric::Ffo, Unit::PerShare, r"^ffo per share$", 1),
            (Metric::Ffo, Unit::Total, r"^ffo$", 1000),
        ],
        None,
    ))?;
    company.tables = crate::table::joined_parentheses;
    Ok(company)
}
