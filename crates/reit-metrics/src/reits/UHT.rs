use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "UHT",
        798783,
        &[
            "uhrit.com",
            "www.uhrit.com",
            "universalhealthrealty.gcs-web.com",
        ],
        vec![(Metric::Ffo, Unit::PerShare, r"^ffo$", 1)],
        None,
    ))?;
    company.tables = tables;
    company.debt = Some(debt);
    Ok(company)
}

pub(crate) fn tables(html: &str) -> Vec<crate::table::Table> {
    let mut tables = crate::table::joined_parentheses(html);
    for table in &mut tables {
        let mut columns = Vec::new();
        for row in &mut table.rows {
            if row.iter().any(|cell| {
                cell.text
                    .to_lowercase()
                    .replace(' ', "")
                    .contains("perdilutedshare")
            }) {
                columns = row
                    .iter()
                    .filter(|cell| {
                        cell.text
                            .to_lowercase()
                            .replace(' ', "")
                            .contains("perdilutedshare")
                    })
                    .cloned()
                    .collect();
            }
            if row.iter().any(|cell| cell.text == "FFO") {
                row.retain(|cell| {
                    cell.text == "FFO" || columns.iter().any(|header| header.covers(cell.column))
                });
            }
        }
    }
    tables
}

fn debt(html: &scraper::Html, as_of: chrono::NaiveDate) -> anyhow::Result<crate::debt::Debt> {
    if chrono::Datelike::month(&as_of) == 12 {
        return crate::debt::tables(
            html,
            as_of,
            &[crate::debt::Rule {
                intro: "aggregate consolidated scheduled debt repayments",
                ..crate::debt::Rule::new(0, 1000)
            }],
            None,
        );
    }
    crate::debt::tables(
        html,
        as_of,
        &[
            crate::debt::Rule {
                anchor: "",
                heading: "Maturity Date, Year Ending December 31",
                row: Some(("Debt(a)", 0)),
                ..crate::debt::Rule::new(0, 1000)
            },
            crate::debt::Rule {
                anchor: "",
                heading: "Maturity Date, Year Ending December 31",
                row: Some(("Debt(b)", 0)),
                ..crate::debt::Rule::new(0, 1000)
            },
        ],
        None,
    )
}
