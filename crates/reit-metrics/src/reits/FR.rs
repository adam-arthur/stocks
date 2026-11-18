use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;
pub fn company() -> Result<Company> {
    let mut company = build((
        "FR",
        921825,
        &[],
        vec![
            (
                Metric::Affo,
                Unit::Total,
                r#"^adjusted funds from operations(?: \([“"]affo[”"]\))?$"#,
                1000,
            ),
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^ffo (?:basic/|basic and )?diluted per share/unit$",
                1,
            ),
            (
                Metric::Ffo,
                Unit::Total,
                r"^ffo allocable to common stockholders and unitholders$",
                1000,
            ),
        ],
        None,
    ))?;
    company.tables = |html| {
        let mut tables = crate::table::joined_parentheses(html);
        for table in &mut tables {
            let mut ffo = false;
            for cell in table
                .rows
                .iter_mut()
                .filter_map(|row| row.iter_mut().find(|cell| !cell.text.is_empty()))
            {
                let label = cell.text.to_lowercase();
                if label.contains("per share/unit") && ffo {
                    cell.text = format!("FFO {}", cell.text);
                } else if !label.starts_with("basic per share/unit") {
                    ffo = label.starts_with("ffo")
                        && label.contains("allocable to common stockholders and unitholders");
                }
            }
        }
        tables
    };
    company.affo_support = crate::AffoSupport::Reported;
    company.debt = Some(debt);
    Ok(company)
}

fn debt(html: &scraper::Html, as_of: chrono::NaiveDate) -> anyhow::Result<crate::debt::Debt> {
    crate::debt::tables(html, as_of, &[crate::debt::Rule::new(0, 1000)], None)
}
