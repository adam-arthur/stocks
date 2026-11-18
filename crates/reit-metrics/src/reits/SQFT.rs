use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "SQFT",
        1080657,
        &["presidiopt.com", "www.presidiopt.com", "ir.presidiopt.com"],
        vec![(Metric::Ffo, Unit::Total, r"^ffo$", 1)],
        None,
    ))?;
    company.filings = &["8-K", "8-K/A", "10-Q", "10-Q/A", "10-K", "10-K/A"];
    company.tables = |html| {
        let mut tables = crate::table::joined_parentheses(html);
        for table in &mut tables {
            if !table
                .rows
                .iter()
                .take(4)
                .flatten()
                .any(|c| c.text == "06/30/22")
            {
                continue;
            }
            for row in &mut table.rows {
                if row.iter().any(|c| c.text == "FFO") {
                    if let Some(cell) = row
                        .iter_mut()
                        .find(|c| crate::number(&c.text) == Some(-46024.0))
                    {
                        cell.text.clear();
                    }
                }
            }
        }
        tables
    };
    company.debt = Some(debt);
    company.affo_support = crate::AffoSupport::Unavailable;
    Ok(company)
}

fn debt(html: &scraper::Html, as_of: chrono::NaiveDate) -> anyhow::Result<crate::debt::Debt> {
    crate::debt::tables(html, as_of, &[crate::debt::Rule::new(2, 1)], None)
}
