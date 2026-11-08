use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "EPR",
        1045450,
        &["investors.eprkc.com"],
        vec![
            (
                Metric::Ffo,
                Unit::PerShare,
                r#"^ffo per (?:diluted common share|common share: diluted)$"#,
                1,
            ),
            (
                Metric::Ffo,
                Unit::Total,
                r#"^ffo available to common shareholders of epr properties$"#,
                1000,
            ),
        ],
        None,
    ))?;
    company.tables = crate::table::joined_parentheses;
    Ok(company)
}
