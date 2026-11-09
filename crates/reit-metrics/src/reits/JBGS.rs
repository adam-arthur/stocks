use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "JBGS",
        1689796,
        &["www.jbgsmith.com", "investors.jbgsmith.com"],
        vec![
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^ffo per (?:common share\s*[-–—]\s*diluted|diluted common share)$",
                1,
            ),
            (
                Metric::Ffo,
                Unit::Total,
                r"^ffo attributable to common shareholders$",
                1000,
            ),
        ],
        None,
    ))?;
    company.debt = Some(debt);
    Ok(company)
}

fn debt(html: &scraper::Html, as_of: chrono::NaiveDate) -> anyhow::Result<crate::debt::Debt> {
    crate::debt::tables(html, as_of, &[crate::debt::Rule::new(0, 1000)], None)
}
