use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "UE",
        1611547,
        &["uedge.com", "www.uedge.com", "investors.uedge.com"],
        vec![(
            Metric::Ffo,
            Unit::PerShare,
            r"^ffo per diluted (?:common )?share$",
            1,
        )],
        None,
    ))?;
    company.debt = Some(debt);
    Ok(company)
}

fn debt(html: &scraper::Html, as_of: chrono::NaiveDate) -> anyhow::Result<crate::debt::Debt> {
    crate::debt::tables(html, as_of, &[crate::debt::Rule::new(0, 1000)], None)
}
