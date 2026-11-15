use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "PEB",
        1474098,
        &[
            "pebblebrookhotels.com",
            "www.pebblebrookhotels.com",
            "investor.pebblebrookhotels.com",
        ],
        vec![
            (
                Metric::Affo,
                Unit::PerShare,
                r"^adjusted ffo per common share - diluted$",
                1,
            ),
            (
                Metric::Affo,
                Unit::Total,
                r"^adjusted ffo available to common share and unit holders$",
                1000,
            ),
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^ffo per common share\s*[-–—]\s*diluted$",
                1,
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
