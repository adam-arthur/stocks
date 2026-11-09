use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "WPC",
        1025378,
        &["wpcarey.com", "www.wpcarey.com", "ir.wpcarey.com"],
        vec![
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^ffo (?:\(as defined by nareit\) )?attributable to w\. p\. carey (?:\(as defined by nareit\) )?per diluted share$",
                1,
            ),
            (
                Metric::Ffo,
                Unit::Total,
                r"^ffo (?:\(as defined by nareit\) )?attributable to w\. p\. carey(?: \(as defined by nareit\))?$",
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
