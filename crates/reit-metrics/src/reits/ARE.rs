use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "ARE",
        1035443,
        &["www.are.com", "investor.are.com"],
        vec![
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^(?:ffo|funds from operations) per share attributable to alexandria['’]s common stockholders [–—-] diluted$",
                1,
            ),
            (
                Metric::Ffo,
                Unit::Total,
                r"^(?:ffo|funds from operations) attributable to alexandria['’]s common stockholders [–—-] diluted$",
                1000,
            ),
        ],
        Some("per share attributable to alexandria"),
    ))?;
    company.debt = Some(debt);
    Ok(company)
}

fn debt(html: &scraper::Html, as_of: chrono::NaiveDate) -> anyhow::Result<crate::debt::Debt> {
    crate::debt::tables(
        html,
        as_of,
        &[crate::debt::Rule {
            row: Some(("Unsecured debt weighted-average", 0)),
            ..crate::debt::Rule::new(0, 1000)
        }],
        None,
    )
}
