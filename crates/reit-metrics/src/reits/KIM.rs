use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "KIM",
        879101,
        &["www.kimcorealty.com", "investors.kimcorealty.com"],
        vec![
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^ffo per common share\s*[-–—]\s*diluted$",
                1,
            ),
            (
                Metric::Ffo,
                Unit::Total,
                r"^(?:ffo|funds from operations) available to (?:the company[\x27’]s )?common shareholders$",
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
            intro: "consolidated debt maturities",
            row: Some(("Principal (", 0)),
            ..crate::debt::Rule::new(0, 1_000_000)
        }],
        None,
    )
}
