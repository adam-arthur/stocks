use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "RHP",
        1040829,
        &["www.rymanhp.com", "rymanhp.com", "ir.rymanhp.com"],
        vec![(
            Metric::Ffo,
            Unit::PerShare,
            r"^ffo(?: available to common (?:shareholders|stockholders)(?: and unit holders)?)? per diluted share(?:/unit)?$",
            1,
        )],
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
        &[crate::debt::Rule {
            row: Some(("Total", 0)),
            ..crate::debt::Rule::new(0, 1000)
        }],
        None,
    )
}
