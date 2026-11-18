use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "MAA",
        912595,
        &["ir.maac.com", "www.prnewswire.com"],
        vec![
            (
                Metric::Affo,
                Unit::PerShare,
                r"^core (?:affo|adjusted funds from operations) per share(?:\s*[-–—]\s*diluted)?$",
                1,
            ),
            (
                Metric::Affo,
                Unit::Total,
                r"^core (?:adjusted funds from operations|affo attributable to common shareholders and unitholders)$",
                1000,
            ),
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^(?:ffo|funds from operations(?: ffo)?) per share\s*[-–—]\s*diluted$",
                1,
            ),
            (
                Metric::Ffo,
                Unit::Total,
                r"^ffo attributable to common shareholders and unitholders$",
                1000,
            ),
        ],
        None,
    ))?;
    company.tables = crate::table::joined_parentheses;
    company.rules[1].scale = None;
    company.affo_support = crate::AffoSupport::Reported;
    company.debt = Some(debt);
    Ok(company)
}

fn debt(html: &scraper::Html, as_of: chrono::NaiveDate) -> anyhow::Result<crate::debt::Debt> {
    crate::debt::tables(html, as_of, &[crate::debt::Rule::new(0, 1000)], None)
}
