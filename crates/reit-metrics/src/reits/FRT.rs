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
