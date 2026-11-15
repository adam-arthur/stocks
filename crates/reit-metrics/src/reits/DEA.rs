use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "DEA",
        1622194,
        &[],
        vec![
            (
                Metric::Affo,
                Unit::Total,
                r"^cash available for distribution$",
                1000,
            ),
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^ffo,? per share\s*[-–—]\s*(?:weighted average )?fully diluted basis$",
                1,
            ),
            (
                Metric::Ffo,
                Unit::Total,
                r"^(?:funds from operations|ffo)$",
                1000,
            ),
        ],
        None,
    ))?;
    company.tables = tables;
    company.affo_support = crate::AffoSupport::Reported;
    company.sections.push(affo_per_share);
    company.debt = Some(debt);
    Ok(company)
}

fn tables(html: &str) -> Vec<crate::table::Table> {
    crate::table::joined_parentheses(html)
        .into_iter()
        .filter(|table| {
            !table.rows.iter().flatten().any(|cell| {
                cell.text
                    .to_lowercase()
                    .starts_with("cash available for distribution")
            }) || table.rows.iter().flatten().any(|cell| {
                cell.text
                    .to_lowercase()
                    .starts_with("weighted average common shares outstanding")
            })
        })
        .map(|mut table| {
            for row in 1..table.rows.len() {
                if table.rows[row - 1].iter().any(|cell| {
                    cell.text
                        .eq_ignore_ascii_case("weighted average common shares outstanding -")
                }) {
                    for cell in &mut table.rows[row] {
                        if cell.text.eq_ignore_ascii_case("fully diluted basis") {
                            cell.text =
                                "Weighted average common shares outstanding - fully diluted basis"
                                    .into();
                        }
                    }
                }
            }
            table
        })
        .collect()
}

fn affo_per_share(tables: &[crate::table::Table], source: &str) -> Result<Vec<crate::Observation>> {
    crate::affo_per_share(
        company()?,
        tables,
        source,
        crate::companies::Rule {
            metric: Metric::Affo,
            unit: Unit::Total,
            label: regex::Regex::new(
                r"^weighted average common shares outstanding - fully diluted basis$",
            )?,
            scale: Some(1),
            section: None,
        },
    )
}

fn debt(html: &scraper::Html, as_of: chrono::NaiveDate) -> anyhow::Result<crate::debt::Debt> {
    use anyhow::Context;
    // Mortgage balances and final dates omit intervening amortization; do not allocate the entire balance to maturity.
    crate::debt::tables(html, as_of, &[crate::debt::Rule::new(0, 1000)], None)
        .context("DEA needs scheduled mortgage principal payments by year")
}
