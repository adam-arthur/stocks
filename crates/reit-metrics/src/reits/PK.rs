use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "PK",
        1617406,
        &[
            "pkhotelsandresorts.com",
            "www.pkhotelsandresorts.com",
            "investors.pkhotelsandresorts.com",
        ],
        vec![
            (
                Metric::Affo,
                Unit::PerShare,
                r"^adjusted ffo (?:per share\s*[-–—]\s*diluted|attributable to stockholders per diluted share)$",
                1,
            ),
            (
                Metric::Affo,
                Unit::Total,
                r"^adjusted ffo attributable to stockholders$",
                1000000,
            ),
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^(?:nareit )?ffo per share\s*[-–—]\s*diluted$",
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
    crate::debt::tables(html, as_of, &[crate::debt::Rule::new(0, 1000000)], None)
}
