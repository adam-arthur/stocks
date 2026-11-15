use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "NTST",
        1798100,
        &[
            "investors.netstreit.com",
            "www.netstreit.com",
            "netstreit.com",
        ],
        vec![
            (
                Metric::Affo,
                Unit::PerShare,
                r"^affo per common share(?: outstanding)?(?:,|\s*[-–—]) diluted$",
                1,
            ),
            (
                Metric::Affo,
                Unit::PerShare,
                r"^adjusted funds from operations per diluted share$",
                1,
            ),
            (
                Metric::Affo,
                Unit::Total,
                r"^(?:affo|adjusted funds from operations(?: \(affo\))?)$",
                1000,
            ),
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^ffo per common share, diluted$",
                1,
            ),
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^funds from operations per diluted share$",
                1,
            ),
            (Metric::Ffo, Unit::Total, r"^ffo$", 1000),
        ],
        None,
    ))?;
    company.tables = crate::table::joined_parentheses;
    company.debt = Some(debt);
    company.affo_support = crate::AffoSupport::Reported;
    Ok(company)
}

fn debt(html: &scraper::Html, as_of: chrono::NaiveDate) -> anyhow::Result<crate::debt::Debt> {
    crate::debt::tables(html, as_of, &[crate::debt::Rule::new(2, 1000)], None)
}
