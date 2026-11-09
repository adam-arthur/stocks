use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "NHP",
        1561032,
        &["investors.nhpreit.com", "nhpreit.com", "www.nhpreit.com"],
        vec![
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^ffo per (?:common )?share\s*[-–—]\s*diluted$",
                1,
            ),
            (
                Metric::Ffo,
                Unit::Total,
                r"^ffo \(as defined by nareit\) attributable to common stockholders$",
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
    crate::debt::tables(html, as_of, &[crate::debt::Rule::new(4, 1000)], None)
}
