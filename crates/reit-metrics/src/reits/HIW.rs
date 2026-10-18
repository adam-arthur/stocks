use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "HIW",
        921082,
        &[],
        vec![
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^funds from operations available for common stockholders per share$",
                1,
            ),
            (
                Metric::Ffo,
                Unit::Total,
                r"^funds from operations available for common stockholders$",
                1000,
            ),
        ],
        None,
    ))?;
    company.filings = &["10-Q", "10-K"];
    company.derive = Some(|documents| crate::derive::fourth_quarters("HIW", documents));
    company.preserve_units = true;
    company.tables = crate::table::joined_parentheses;
    Ok(company)
}
