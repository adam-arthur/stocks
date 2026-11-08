use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "ELME",
        104894,
        &["ir.elmecommunities.com"],
        vec![
            (
                Metric::Ffo,
                Unit::PerShare,
                r#"^nareit ffo per diluted share$"#,
                1,
            ),
            (
                Metric::Ffo,
                Unit::Total,
                r#"^nareit funds from operations$"#,
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
            let mut nareit = false;
            for row in &mut table.rows {
                if let Some(cell) = row.iter_mut().find(|cell| !cell.text.is_empty()) {
                    if cell.text.eq_ignore_ascii_case("(Diluted)") && nareit {
                        cell.text = "NAREIT FFO per diluted share".into();
                        nareit = false;
                    } else {
                        nareit = cell.text.eq_ignore_ascii_case("NAREIT FFO");
                    }
                }
            }
            table
        })
        .collect()
}
