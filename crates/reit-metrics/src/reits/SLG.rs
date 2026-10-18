use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "SLG",
        1040971,
        &["slgreen.com", "www.slgreen.com", "slgreen.gcs-web.com"],
        vec![(Metric::Ffo, Unit::PerShare, r"^diluted ffo per share$", 1)],
        None,
    ))?;
    company.tables = tables;
    Ok(company)
}

fn tables(html: &str) -> Vec<crate::table::Table> {
    crate::table::joined_parentheses(html)
        .into_iter()
        .map(|mut table| {
            for cell in table.rows.iter_mut().flatten() {
                if cell
                    .text
                    .to_lowercase()
                    .starts_with("ffo per share (diluted)")
                {
                    cell.text = "Diluted FFO per share".into();
                }
            }
            table
        })
        .collect()
}
