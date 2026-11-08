use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "CUZ",
        25232,
        &[],
        vec![
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^ffo per share(?: - basic and diluted)?$",
                1,
            ),
            (Metric::Ffo, Unit::Total, r"^ffo$", 1000),
        ],
        None,
    ))?;
    company.tables = tables;
    Ok(company)
}

fn tables(html: &str) -> Vec<crate::table::Table> {
    let quarter = regex::Regex::new(r"^(20\d{2}) ([1-4])(?:st|nd|rd|th)$").unwrap();
    crate::table::joined_parentheses(html)
        .into_iter()
        .map(|mut table| {
            for cell in table.rows.iter_mut().take(8).flatten() {
                if let Some(period) = quarter.captures(&cell.text) {
                    cell.text = format!("Q{} {}", &period[2], &period[1]);
                }
            }
            table
        })
        .collect()
}
