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
