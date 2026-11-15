use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "NLOP",
        1952976,
        &["nloproperties.com", "www.nloproperties.com"],
        vec![
            (
                Metric::Affo,
                Unit::PerShare,
                r"^affo attributable to nlop per diluted share$",
                1,
            ),
            (
                Metric::Affo,
                Unit::Total,
                r"^affo attributable to nlop$",
                1000,
            ),
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^ffo \(as defined by nareit\) attributable to nlop per diluted share$",
                1,
            ),
            (
                Metric::Ffo,
                Unit::Total,
                r"^ffo \(as defined by nareit\) attributable to nlop$",
                1000,
            ),
        ],
        None,
    ))?;
    company.tables = crate::table::joined_parentheses;
    company.additional_items = Some(|items| items.split(',').any(|item| item.trim() == "7.01"));
    company.affo_support = crate::AffoSupport::Reported;
    company.filings = &["8-K", "8-K/A", "10-Q", "10-Q/A", "10-K", "10-K/A"];
    company.debt = Some(debt);
    Ok(company)
}

fn debt(html: &scraper::Html, as_of: chrono::NaiveDate) -> anyhow::Result<crate::debt::Debt> {
    crate::debt::tables(html, as_of, &[crate::debt::Rule::new(0, 1000)], None)
}
