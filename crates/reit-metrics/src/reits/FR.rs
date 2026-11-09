use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;
pub fn company() -> Result<Company> {
    let mut company = build((
        "FR",
        921825,
        &[],
        vec![
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^(?:basic/|basic and )?diluted per share/unit$",
                1,
            ),
            (
                Metric::Ffo,
                Unit::Total,
                r"^ffo allocable to common stockholders and unitholders$",
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
