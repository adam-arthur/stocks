use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "FVR",
        1988494,
        &[],
        vec![
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^ffo per share(?:, diluted)?$",
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
    Ok(company)
}

fn tables(html: &str) -> Vec<crate::table::Table> {
    crate::table::joined_parentheses(html)
        .into_iter()
        .map(|mut table| {
            let pro_forma = table
                .rows
                .iter()
                .take(8)
                .flatten()
                .filter(|cell| cell.text.to_lowercase().contains("pro forma"))
                .cloned()
                .collect::<Vec<_>>();
            for cell in table.rows.iter_mut().flatten() {
                if pro_forma.iter().any(|header| header.covers(cell.column))
                    && (crate::number(&cell.text).is_some()
                        || cell.text.to_lowercase().contains("pro forma"))
                {
                    cell.text.clear();
                }
            }
            table
        })
        .collect()
}
