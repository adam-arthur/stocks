use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "DOC",
        765880,
        &["healthpeak.com", "www.healthpeak.com", "ir.healthpeak.com"],
        vec![(
            Metric::Ffo,
            Unit::PerShare,
            r"^diluted (?:nareit )?ffo per common share$",
            1,
        )],
        None,
    ))?;
    company.tables = crate::table::joined_parentheses;
    company.debt = Some(debt);
    Ok(company)
}

fn debt(html: &scraper::Html, as_of: chrono::NaiveDate) -> anyhow::Result<crate::debt::Debt> {
    crate::debt::tables(html, as_of, &[crate::debt::Rule::new(7, 1000)], None)
}
