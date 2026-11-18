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
    company.tables = |html| {
        let mut tables = crate::table::joined_parentheses(html);
        for table in &mut tables {
            if let Some(core) = table.rows.iter().position(|row| {
                row.iter().any(|c| {
                    c.text
                        .to_lowercase()
                        .starts_with("core funds from operations")
                })
            }) {
                if table.rows[core + 1..].iter().any(|row| {
                    row.iter()
                        .any(|c| c.text.eq_ignore_ascii_case("Funds From Operations"))
                }) {
                    for row in &mut table.rows[..core] {
                        if row
                            .iter()
                            .any(|c| c.text.eq_ignore_ascii_case("Funds From Operations"))
                        {
                            row.clear();
                        }
                    }
                }
            }
        }
        tables
            .into_iter()
            .filter(|table| {
                !table.rows.iter().take(5).any(|row| {
                    row.iter().any(|cell| cell.text == "IRT")
                        && row.iter().any(|cell| cell.text == "STAR")
                })
            })
            .collect()
    };
    company.affo_support = crate::AffoSupport::Unavailable;
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
