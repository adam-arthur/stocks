use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "ADC",
        917251,
        &[
            "investors.agreerealty.com",
            "www.agreerealty.com",
            "agreerealty.com",
        ],
        vec![
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^(?:ffo|funds from operations) per (?:common )?share(?: and (?:op|partnership) unit)?\s*[-–—]\s*diluted$",
                1,
            ),
            (
                Metric::Ffo,
                Unit::Total,
                r"^(?:ffo|funds from operations)\s*[-–—]\s*operating partnership common unitholders$",
                1000,
            ),
        ],
        None,
    ))?;
    company.debt = Some(debt);
    Ok(company)
}

fn debt(html: &scraper::Html, as_of: chrono::NaiveDate) -> anyhow::Result<crate::debt::Debt> {
    use chrono::Datelike;
    let quarterly = as_of.month() != 12;
    crate::debt::tables(
        html,
        as_of,
        &[crate::debt::Rule::new(if quarterly { 0 } else { 2 }, 1000)],
        None,
    )
}
