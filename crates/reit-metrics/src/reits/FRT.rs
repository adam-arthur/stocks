use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;
pub fn company() -> Result<Company> {
    let mut company = build((
        "FRT",
        34903,
        &[],
        vec![
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^(?:nareit )?ffo per diluted share$",
                1,
            ),
            (Metric::Ffo, Unit::Total, r"^nareit ffo$", 1000),
            (
                Metric::Ffo,
                Unit::Total,
                r"^funds from operations available for common shareholders$",
                1000,
            ),
        ],
        None,
    ))?;
    company.tables = tables;
    company.debt = Some(debt);
    Ok(company)
}

fn tables(html: &str) -> Vec<crate::table::Table> {
    crate::table::joined_parentheses(html)
        .into_iter()
        .filter(|table| {
            table.rows.iter().any(|row| {
                row.iter()
                    .find(|cell| !cell.text.is_empty())
                    .is_some_and(|cell| cell.text.to_lowercase().starts_with("net income"))
            })
        })
        .collect()
}

fn debt(html: &scraper::Html, as_of: chrono::NaiveDate) -> anyhow::Result<crate::debt::Debt> {
    crate::debt::tables(
        html,
        as_of,
        &[crate::debt::Rule {
            anchor: "",
            intro: if chrono::Datelike::month(&as_of) == 12 {
                "Scheduled principal payments on mortgages payable"
            } else {
                "summary of our scheduled principal repayments"
            },
            ..crate::debt::Rule::new(usize::MAX, 1000)
        }],
        None,
    )
}
