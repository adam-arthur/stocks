use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;
pub fn company() -> Result<Company> {
    let mut company = build((
        "LINE",
        1868159,
        &[],
        vec![(Metric::Ffo, Unit::Total, r"^ffo$", 1000000)],
        None,
    ))?;
    company.tables = tables;
    Ok(company)
}

fn tables(html: &str) -> Vec<crate::table::Table> {
    let mut tables = crate::table::joined_parentheses(html);
    for table in &mut tables {
        if table
            .rows
            .iter()
            .flatten()
            .any(|cell| cell.text.trim() == "FFO")
            && table
                .rows
                .iter()
                .take(4)
                .flatten()
                .filter(|cell| crate::duration(&cell.text).is_some())
                .count()
                == 1
        {
            for cell in table.rows.iter_mut().take(4).flatten() {
                if cell.text.to_lowercase().starts_with("three months ended") {
                    cell.column = 0;
                    cell.width = usize::MAX;
                }
            }
        }
    }
    tables
}
