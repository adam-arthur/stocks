use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;
use regex::Regex;

pub fn company() -> Result<Company> {
    let mut company = build((
        "AMH",
        1562401,
        &["investors.amh.com", "www.amh.com"],
        vec![
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^ffo attributable to common share and unit holders$",
                1,
            ),
            (
                Metric::Ffo,
                Unit::Total,
                r"^ffo attributable to common share and unit holders$",
                1000,
            ),
        ],
        None,
    ))?;
    company.rules[0].section = Some(Regex::new(r"(?i)^per ffo share and unit:$")?);
    company.rules[1].section = Some(Regex::new(r"^$")?);
    company.debt = Some(debt);
    Ok(company)
}

fn debt(html: &scraper::Html, as_of: chrono::NaiveDate) -> anyhow::Result<crate::debt::Debt> {
    crate::debt::tables(html, as_of, &[crate::debt::Rule::new(0, 1000)], None)
}
