use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

const FFO: &str = r"(?i)^ffo(?: attributable to common stockholders and unit holders)?$";

pub fn company() -> Result<Company> {
    let mut company = build((
        "EXR",
        1289490,
        &[],
        vec![(Metric::Ffo, Unit::PerShare, FFO, 1)],
        None,
    ))?;
    company.tables = tables;
    company.affo_support = crate::AffoSupport::Unavailable;
    company.debt = Some(debt);
    Ok(company)
}

fn tables(html: &str) -> Vec<crate::table::Table> {
    let mut tables = crate::table::joined_parentheses(html);
    let label = regex::Regex::new(FFO).unwrap();
    for table in &mut tables {
        let columns: Vec<_> = table
            .rows
            .iter()
            .flatten()
            .filter(|cell| cell.text.to_lowercase().contains("(per share)"))
            .cloned()
            .collect();
        for row in &mut table.rows {
            if row.iter().any(|cell| label.is_match(&cell.text)) {
                row.retain(|cell| {
                    label.is_match(&cell.text)
                        || columns.iter().any(|header| header.covers(cell.column))
                });
            }
        }
    }
    tables
}

fn debt(html: &scraper::Html, as_of: chrono::NaiveDate) -> Result<crate::debt::Debt> {
    use crate::debt::{Rule, add_payments, amount, rows, scalar, tables};
    use anyhow::{Context, ensure};
    use chrono::Datelike;

    let term = tables(
        html,
        as_of,
        &[Rule {
            intro: "scheduled maturities of term debt",
            ..Rule::new(0, 1000)
        }],
        None,
    )?;
    let mut payments = Vec::new();
    for row in rows(
        html,
        as_of,
        "Revolving Lines of Credit and Commercial Paper",
    )? {
        let Some(label) = row.first() else { continue };
        if !(label.contains("credit line") || label == "Commercial paper") || label.starts_with('(')
        {
            continue;
        }
        let drawn = row
            .get(1)
            .and_then(|cell| amount(cell, 1000))
            .context("missing drawn balance")?;
        if drawn > 0 {
            let date = row
                .iter()
                .find_map(|cell| chrono::NaiveDate::parse_from_str(cell, "%m/%d/%Y").ok())
                .with_context(|| {
                    format!("{label}: ${drawn} outstanding without a contractual maturity date")
                })?;
            ensure!(date >= as_of, "past-due {label}");
            payments.push((date.year(), drawn));
        }
    }
    ensure!(
        scalar(html, "us-gaap:LineOfCredit", as_of)?
            == Some(i64::try_from(
                payments.iter().map(|(_, amount)| amount).sum::<u64>()
            )?),
        "credit line and commercial paper payments do not reconcile to their reported total"
    );
    add_payments(term, &payments)
}
