use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "NXRT",
        1620393,
        &["nxrt.nexpoint.com", "www.nexpointliving.com"],
        vec![
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^ffo per share\s*[-–—]\s*diluted$",
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
    company.tables = |html| {
        crate::table::joined_parentheses(html)
            .into_iter()
            .filter(|table| {
                !table.rows.iter().take(6).flatten().any(|cell| {
                    cell.text.to_lowercase().contains("mid-point")
                        || cell.text.to_lowercase().contains("midpoint")
                })
            })
            .collect()
    };
    company.debt = Some(debt);
    Ok(company)
}

fn debt(html: &scraper::Html, as_of: chrono::NaiveDate) -> anyhow::Result<crate::debt::Debt> {
    crate::debt::tables(
        html,
        as_of,
        &[crate::debt::Rule {
            anchor: "us-gaap:LongTermDebtMaturitiesRepaymentsOfPrincipalAfterYearFive",
            ..crate::debt::Rule::new(2, 1000)
        }],
        None,
    )
}
