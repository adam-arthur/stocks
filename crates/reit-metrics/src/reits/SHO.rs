use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "SHO",
        1295810,
        &["www.sunstonehotels.com", "sunstonehotels.com"],
        vec![
            (
                Metric::Affo,
                Unit::PerShare,
                r"^adjusted ffo attributable to common stockholders per diluted share$",
                1,
            ),
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^ffo attributable to common stockholders per diluted share$",
                1,
            ),
        ],
        None,
    ))?;
    company.tables = |html| {
        let mut tables = crate::table::joined_parentheses(html);
        for table in &mut tables {
            if let Some((i, actual)) = table.rows.iter().take(8).enumerate().find_map(|(i, row)| {
                row.iter()
                    .any(|cell| cell.text.to_lowercase().contains("forma"))
                    .then(|| {
                        row.iter()
                            .find(|cell| cell.text.to_lowercase().starts_with("actual"))
                            .cloned()
                            .map(|cell| (i, cell))
                    })
                    .flatten()
            }) {
                for cell in table.rows.iter_mut().skip(i + 1).flatten() {
                    if crate::number(&cell.text).is_some() && !actual.covers(cell.column) {
                        cell.text.clear();
                    }
                }
            }
        }
        tables
    };
    company.debt = Some(debt);
    company.affo_support = crate::AffoSupport::Reported;
    Ok(company)
}

fn debt(html: &scraper::Html, as_of: chrono::NaiveDate) -> anyhow::Result<crate::debt::Debt> {
    crate::debt::bullets(
        html,
        as_of,
        "Total unsecured corporate credit facilities",
        (r"^(Term Loan [0-9]+|Series [AB])$", 4, 3),
        ("Total debt", &[]),
    )
}
