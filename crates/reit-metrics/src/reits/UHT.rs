use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "UHT",
        798783,
        &[
            "uhrit.com",
            "www.uhrit.com",
            "universalhealthrealty.gcs-web.com",
        ],
        vec![(Metric::Ffo, Unit::PerShare, r"^ffo$", 1)],
        None,
    ))?;
    company.tables = tables;
    Ok(company)
}

pub(crate) fn tables(html: &str) -> Vec<crate::table::Table> {
    let mut tables = crate::table::joined_parentheses(html);
    for table in &mut tables {
        let mut columns = Vec::new();
        for row in &mut table.rows {
            if row.iter().any(|cell| {
                cell.text
                    .to_lowercase()
                    .replace(' ', "")
                    .contains("perdilutedshare")
            }) {
                columns = row
                    .iter()
                    .filter(|cell| {
                        cell.text
                            .to_lowercase()
                            .replace(' ', "")
                            .contains("perdilutedshare")
                    })
                    .cloned()
                    .collect();
            }
            if row.iter().any(|cell| cell.text == "FFO") {
                row.retain(|cell| {
                    cell.text == "FFO" || columns.iter().any(|header| header.covers(cell.column))
                });
            }
        }
    }
    tables
}
