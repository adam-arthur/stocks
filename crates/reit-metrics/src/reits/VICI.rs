use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "VICI",
        1705696,
        &["www.viciproperties.com", "investors.viciproperties.com"],
        vec![
            (
                Metric::Affo,
                Unit::PerShare,
                r"^affo per (?:common )?share:? (?:basic and )?diluted$",
                1,
            ),
            (
                Metric::Affo,
                Unit::Total,
                r"^(?:affo(?: attributable to common stockholders)?|adjusted funds from operations)$",
                1000,
            ),
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^ffo per (?:common )?share:? (?:basic and )?diluted$",
                1,
            ),
            (
                Metric::Ffo,
                Unit::Total,
                r"^ffo attributable to common stockholders$",
                1000,
            ),
        ],
        None,
    ))?;
    company.tables = crate::table::joined_parentheses;
    company.debt = Some(debt);
    company.affo_support = crate::AffoSupport::Reported;
    Ok(company)
}

fn debt(html: &scraper::Html, as_of: chrono::NaiveDate) -> anyhow::Result<crate::debt::Debt> {
    crate::debt::tables(html, as_of, &[crate::debt::Rule::new(0, 1000)], None)
}
