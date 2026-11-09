use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "SHO",
        1295810,
        &["www.sunstonehotels.com", "sunstonehotels.com"],
        vec![(
            Metric::Ffo,
            Unit::PerShare,
            r"^ffo attributable to common stockholders per diluted share$",
            1,
        )],
        None,
    ))?;
    company.tables = crate::table::joined_parentheses;
    company.debt = Some(debt);
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
