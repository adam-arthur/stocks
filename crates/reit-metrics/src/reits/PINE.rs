use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "PINE",
        1786117,
        &["alpinereit.com", "www.alpinereit.com", "ir.alpinereit.com"][..],
        vec![
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^ffo (?:attributable to common stockholders )?per diluted share$",
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
    company.debt = Some(debt);
    Ok(company)
}

fn debt(html: &scraper::Html, as_of: chrono::NaiveDate) -> anyhow::Result<crate::debt::Debt> {
    crate::debt::tables(
        html,
        as_of,
        &[crate::debt::Rule {
            anchor: "",
            heading: "Total Long-Term Debt - Face Value",
            intro: "Payments applicable to reduction of principal amounts",
            ..crate::debt::Rule::new(0, 1000)
        }],
        None,
    )
}
