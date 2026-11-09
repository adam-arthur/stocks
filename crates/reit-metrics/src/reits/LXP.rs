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
    company.debt = Some(debt);
    Ok(company)
}

fn debt(html: &scraper::Html, as_of: chrono::NaiveDate) -> anyhow::Result<crate::debt::Debt> {
    crate::debt::tables(html, as_of, &[crate::debt::Rule::new(0, 1000)], None)
}
