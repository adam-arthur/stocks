use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "PSTL",
        1759774,
        &[
            "postalrealtytrust.com",
            "www.postalrealtytrust.com",
            "investor.postalrealtytrust.com",
        ],
        vec![
            (Metric::Affo, Unit::Total, r"^affo$", 0),
            (
                Metric::Affo,
                Unit::PerShare,
                r"^affo per (?:share|common share and common unit outstanding(?:\s*[-–—]\s*diluted)?)$",
                1,
            ),
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^ffo per (?:share|common share and common unit outstanding(?:\s*[-–—]\s*diluted)?)$",
                1,
            ),
        ],
        None,
    ))?;
    company.tables = |html| {
        crate::table::joined_parentheses(
            &regex::Regex::new(r"(?i)\(in\s+(thousands|millions)")
                .unwrap()
                .replace_all(html, "(dollars in $1"),
        )
    };
    company.debt = Some(debt);
    company.affo_support = crate::AffoSupport::Reported;
    Ok(company)
}

fn debt(html: &scraper::Html, as_of: chrono::NaiveDate) -> anyhow::Result<crate::debt::Debt> {
    crate::debt::tables(html, as_of, &[crate::debt::Rule::new(0, 1000)], None)
}
