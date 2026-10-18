use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "SBAC",
        1034054,
        &["www.sbasite.com", "ir.sbasite.com"],
        vec![
            (Metric::Ffo, Unit::PerShare, r"^ffo per share$", 1),
            (Metric::Ffo, Unit::Total, r"^ffo$", 1000),
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
            let columns = table
                .rows
                .iter()
                .take(8)
                .flatten()
                .filter(|cell| {
                    cell.text
                        .replace(' ', "")
                        .eq_ignore_ascii_case("($pershare)")
                })
                .cloned()
                .collect::<Vec<_>>();
            if !columns.is_empty() {
                for row in &mut table.rows {
                    if let Some(label) = row.iter().position(|cell| cell.text == "FFO") {
                        row[label].text = "FFO per share".into();
                        for cell in row.iter_mut().skip(label + 1) {
                            if !columns.iter().any(|header| header.covers(cell.column)) {
                                cell.text.clear();
                            }
                        }
                    }
                }
            }
            table
        })
        .collect()
}
