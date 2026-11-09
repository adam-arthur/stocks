use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "MPT",
        1287865,
        &[
            "ir.medicalpropertiestrust.com",
            "www.medicalpropertiestrust.com",
        ],
        vec![
            (Metric::Ffo, Unit::PerShare, r"^funds from operations$", 1),
            (Metric::Ffo, Unit::Total, r"^funds from operations$", 1000),
        ],
        None,
    ))?;
    company.tables = crate::table::joined_parentheses;
    company.rules[0].section = Some(regex::Regex::new(
        r"(?i)^per (?:diluted |common )?share data:?",
    )?);
    company.rules[1].section = Some(regex::Regex::new(r"(?i)^ffo information:?")?);
    company.debt = Some(debt);
    company.ownership = Some(crate::ownership::Rules::default());
    Ok(company)
}

fn debt(html: &scraper::Html, as_of: chrono::NaiveDate) -> anyhow::Result<crate::debt::Debt> {
    crate::debt::tables(html, as_of, &[crate::debt::Rule::new(0, 1000)], None)
}
