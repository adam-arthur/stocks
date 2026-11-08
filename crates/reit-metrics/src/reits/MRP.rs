use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "MRP",
        2017206,
        &["ir.millroseproperties.com", "www.millroseproperties.com"],
        vec![
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^gaap reported basic and diluted earnings per share of class a and class b common stock$",
                1,
            ),
            (
                Metric::Ffo,
                Unit::Total,
                r"^net income attributable to millrose properties, inc\. common stockholders$",
                1000,
            ),
        ],
        None,
    ))?;
    company.filings = &["8-K", "8-K/A", "10-Q", "10-Q/A", "10-K", "10-K/A"];
    company.tables = tables;
    Ok(company)
}

fn tables(html: &str) -> Vec<crate::table::Table> {
    if !crate::table::normalize(
        &scraper::Html::parse_document(html)
            .root_element()
            .text()
            .collect::<Vec<_>>()
            .join(" "),
    )
    .to_lowercase()
    .contains("there were no applicable adjustments to net income of the company to calculate ffo")
    {
        return Vec::new();
    }
    crate::table::joined_parentheses(html)
        .into_iter()
        .filter(|table| {
            table.rows.iter().flatten().any(|cell| {
                cell.text
                    .to_lowercase()
                    .starts_with("affo attributable to millrose")
            })
        })
        .collect()
}
