use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "CTRE",
        1590717,
        &[],
        vec![(
            Metric::Ffo,
            Unit::PerShare,
            r"^ffo (?:per share(?: attributable to caretrust reit, inc\.)?|attributable to caretrust reit, inc\., per share)$",
            1,
        )],
        None,
    ))?;
    company.tables = tables;
    Ok(company)
}

fn tables(html: &str) -> Vec<crate::table::Table> {
    crate::table::joined_parentheses(html)
        .into_iter()
        .map(|mut table| {
            for cell in table.rows.iter_mut().take(8).flatten() {
                if cell.text.eq_ignore_ascii_case("quarter") {
                    cell.text = "Three Months Ended".into();
                }
            }
            table
        })
        .collect()
}
