use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "AHR",
        1632970,
        &[
            "ir.americanhealthcarereit.com",
            "www.americanhealthcarereit.com",
        ],
        vec![
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^(?:nareit )?ffo per common share attributable to controlling interest\s*[-–—]\s*(?:basic and )?diluted$",
                1,
            ),
            (
                Metric::Ffo,
                Unit::Total,
                r"^(?:nareit )?ffo attributable to controlling interest$",
                1000,
            ),
        ],
        None,
    ))?;
    company.additional_items = Some(|items| items.split(',').any(|item| item.trim() == "7.01"));
    company.debt = Some(debt);
    Ok(company)
}

fn debt(html: &scraper::Html, as_of: chrono::NaiveDate) -> anyhow::Result<crate::debt::Debt> {
    crate::debt::tables(
        html,
        as_of,
        &([
            "Fixed-rate debt — principal payments",
            "Variable-rate debt — principal payments",
        ]
        .into_iter()
        .map(|row| crate::debt::Rule {
            anchor: "",
            heading: "Expected Maturity Date",
            row: Some((row, 0)),
            ..crate::debt::Rule::new(0, 1000)
        })
        .collect::<Vec<_>>()),
        None,
    )
}
