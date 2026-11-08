use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;
pub fn company() -> Result<Company> {
    let mut company = build((
        "LXP",
        910108,
        &[],
        vec![
            (Metric::Ffo, Unit::PerShare, r"^ffo$", 1),
            (
                Metric::Ffo,
                Unit::Total,
                r"^ffo available to (?:all equityholders|common shareholders and unitholders) - diluted$",
                1000,
            ),
        ],
        None,
    ))?;
    company.rules[0].section = Some(regex::Regex::new(r"(?i)^diluted:?$")?);
    company.tables = crate::table::joined_parentheses;
    Ok(company)
}
