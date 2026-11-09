use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "CTO",
        23795,
        &["www.ctoreit.com", "ctoreit.com", "ir.ctoreit.com"],
        vec![
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^ffo(?: attributable to common stockholders)? per (?:common share [-–] diluted|diluted share)$",
                1,
            ),
            (
                Metric::Ffo,
                Unit::Total,
                r"^funds from operations attributable to common stockholders$",
                1000,
            ),
        ],
        None,
    ))?;
    company.tables = crate::table::joined_parentheses;
    company.debt = Some(debt);
    Ok(company)
}

fn debt(html: &scraper::Html, as_of: chrono::NaiveDate) -> anyhow::Result<crate::debt::Debt> {
    crate::debt::tables(html, as_of, &[crate::debt::Rule::new(0, 1000)], None)
}
