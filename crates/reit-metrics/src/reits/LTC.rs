use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;
pub fn company() -> Result<Company> {
    let mut company = build((
        "LTC",
        887905,
        &[],
        vec![
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^(?:nareit diluted ffo per common share|(?:nareit diluted|diluted(?: nareit)?) ffo attributable to common stockholders per share)$",
                1,
            ),
            (
                Metric::Ffo,
                Unit::Total,
                r"^(?:nareit )?(?:funds from operations|ffo) attributable to common stockholders$",
                1000,
            ),
        ],
        None,
    ))?;
    company.tables = crate::table::joined_parentheses;
    Ok(company)
}
