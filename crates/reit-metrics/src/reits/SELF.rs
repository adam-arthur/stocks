use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "SELF",
        1031235,
        &["www.globalselfstorage.us", "ir.globalselfstorage.us"],
        vec![(
            Metric::Ffo,
            Unit::PerShare,
            r"^ffo per share(?:\s*[-–—]\s*diluted| attributable to common stockholders)?$",
            1,
        )],
        None,
    ))?;
    company.filings = &["8-K", "8-K/A", "10-Q", "10-Q/A", "10-K", "10-K/A"];
    company.tables = tables;
    Ok(company)
}

fn tables(html: &str) -> Vec<crate::table::Table> {
    let date =
        regex::Regex::new(r"^(?:March|June|September|December) (?:30|31), 20\d{2}$").unwrap();
    crate::table::joined_parentheses(html)
        .into_iter()
        .map(|mut table| {
            if table
                .rows
                .iter()
                .take(6)
                .flatten()
                .any(|cell| cell.text == "Months")
            {
                let headers = table.rows.iter().take(6).cloned().collect::<Vec<_>>();
                for (index, row) in table.rows.iter_mut().take(6).enumerate() {
                    for cell in row.iter_mut().filter(|cell| date.is_match(&cell.text)) {
                        cell.text = headers[..=index]
                            .iter()
                            .flatten()
                            .filter(|header| header.covers(cell.column) && !header.text.is_empty())
                            .map(|header| header.text.as_str())
                            .collect::<Vec<_>>()
                            .join(" ")
                            .replace(" through ", " to ");
                    }
                }
            }
            table
        })
        .collect()
}
