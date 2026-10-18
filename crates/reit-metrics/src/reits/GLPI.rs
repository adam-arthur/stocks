use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "GLPI",
        1575965,
        &[],
        vec![
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^ffo, per diluted common share(?: and op(?:/ltip)? units)?$",
                1,
            ),
            (Metric::Ffo, Unit::Total, r"^funds from operations$", 0),
        ],
        None,
    ))?;
    company.tables = tables;
    Ok(company)
}

fn tables(html: &str) -> Vec<crate::table::Table> {
    crate::table::joined_parentheses(html)
        .into_iter()
        .filter_map(|mut table| {
            let actual = table
                .rows
                .iter()
                .take(6)
                .flatten()
                .filter(|cell| cell.text.ends_with("Actual"))
                .cloned()
                .collect::<Vec<_>>();
            if !actual.is_empty() {
                if table
                    .rows
                    .iter()
                    .take(6)
                    .flatten()
                    .any(|cell| cell.text.contains("Months Ending"))
                {
                    return None;
                }
                for cell in table.rows.iter_mut().flatten() {
                    if cell.text.to_lowercase().contains("guidance")
                        || (crate::number(&cell.text).is_some()
                            && !actual.iter().any(|header| header.covers(cell.column)))
                    {
                        cell.text.clear();
                    }
                }
            }
            Some(table)
        })
        .collect()
}
