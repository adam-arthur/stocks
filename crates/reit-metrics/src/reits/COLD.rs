use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "COLD",
        1455863,
        &["www.americold.com", "ir.americold.com"],
        vec![(
            Metric::Ffo,
            Unit::PerShare,
            r"^nareit ffo(?: available to common shareholders)? - diluted per share$",
            1,
        )],
        None,
    ))?;
    company.tables = tables;
    Ok(company)
}

fn tables(html: &str) -> Vec<crate::table::Table> {
    let mut tables = crate::table::joined_parentheses(html);
    let quarter = regex::Regex::new(r"^Q([1-4]) (\d{2})$").unwrap();
    for cell in tables
        .iter_mut()
        .flat_map(|table| table.rows.iter_mut().flatten())
    {
        if let Some(period) = quarter.captures(&cell.text) {
            cell.text = format!("Q{} 20{}", &period[1], &period[2]);
        }
    }
    tables
}
