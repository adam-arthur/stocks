use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "IRT",
        1466085,
        &["www.irtliving.com", "investors.irtliving.com"],
        vec![
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^ffo per share(?:\s*[-–—]+\s*diluted)?$",
                1,
            ),
            (
                Metric::Ffo,
                Unit::Total,
                r"^(?:ffo|funds from operations)$",
                1000,
            ),
        ],
        Some("funds from operations"),
    ))?;
    company.tables = crate::table::joined_parentheses;
    company.debt = Some(debt);
    Ok(company)
}

fn debt(html: &scraper::Html, as_of: chrono::NaiveDate) -> anyhow::Result<crate::debt::Debt> {
    crate::debt::tables(
        html,
        as_of,
        &[crate::debt::Rule {
            row: Some(("Total", 0)),
            ..crate::debt::Rule::new(0, 1000)
        }],
        None,
    )
}
