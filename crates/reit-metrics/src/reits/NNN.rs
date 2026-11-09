use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "NNN",
        751364,
        &["www.nnnreit.com", "nnnreit.com"],
        vec![
            (Metric::Ffo, Unit::PerShare, r"^ffo per share:? diluted$", 1),
            (Metric::Ffo, Unit::PerShare, r"^ffo per share$", 1),
            (Metric::Ffo, Unit::Total, r"^ffo$", 1000),
        ],
        None,
    ))?;
    company.tables = crate::table::joined_parentheses;
    company.debt = Some(debt);
    Ok(company)
}

fn debt(html: &scraper::Html, as_of: chrono::NaiveDate) -> anyhow::Result<crate::debt::Debt> {
    crate::debt::tables(
        html,
        as_of,
        &(["Long-term debt(", "Credit Facility"]
            .into_iter()
            .chain(
                html.select(&scraper::Selector::parse("td, th").unwrap())
                    .any(|cell| {
                        crate::table::normalize(&cell.text().collect::<String>()) == "Term Loan"
                    })
                    .then_some("Term Loan"),
            )
            .map(|row| crate::debt::Rule {
                anchor: "",
                heading: "Total contractual cash obligations",
                row: Some((row, 1)),
                ..crate::debt::Rule::new(0, 1000)
            })
            .collect::<Vec<_>>()),
        None,
    )
}
