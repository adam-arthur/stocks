use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "VNO",
        899689,
        &["vno.com", "www.vno.com", "investors.vno.com"],
        vec![(
            Metric::Ffo,
            Unit::Total,
            r"^ffo attributable to common shareholders plus assumed conversions(?: \(non-gaap\))?$",
            1000,
        )],
        None,
    ))?;
    company.tables = tables;
    Ok(company)
}

pub(crate) fn tables(html: &str) -> Vec<crate::table::Table> {
    crate::table::tables(html)
        .into_iter()
        .filter(|table| {
            !table
                .rows
                .iter()
                .take(6)
                .flatten()
                .any(|cell| cell.text.to_lowercase().contains("financial highlights"))
        })
        .map(|mut table| {
            for cell in table.rows.iter_mut().flatten() {
                match cell.text.to_lowercase().as_str() {
                    "for the quarters" => cell.text = "Three Months Ended".into(),
                    "for the years" => cell.text = "Year Ended".into(),
                    _ => {}
                }
            }
            table
        })
        .collect()
}
