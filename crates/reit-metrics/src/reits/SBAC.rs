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
    company.debt = Some(debt);
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

fn debt(html: &scraper::Html, as_of: chrono::NaiveDate) -> anyhow::Result<crate::debt::Debt> {
    if chrono::Datelike::month(&as_of) != 12 {
        return crate::debt::tables(
            html,
            as_of,
            &[crate::debt::Rule {
                anchor: "",
                intro: "future principal payment obligations and fair values",
                row: Some(("Total debt obligation", 0)),
                ..crate::debt::Rule::new(0, 1000)
            }],
            None,
        );
    }
    crate::debt::tables(
        html,
        as_of,
        &[crate::debt::Rule::new(0, 1000)],
        crate::debt::scalar(html, "us-gaap:DebtInstrumentFaceAmount", as_of)?
            .map(u64::try_from)
            .transpose()?,
    )
}
