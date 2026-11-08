use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "OHI",
        888491,
        &["www.omegahealthcare.com", "omegahealthcare.com"],
        vec![
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^(?:nareit )?funds from operations available per share$",
                1,
            ),
            (Metric::Ffo, Unit::Total, r"^nareit ffo$", 1000),
        ],
        None,
    ))?;
    company.tables = crate::table::joined_parentheses;
    Ok(company)
}
