use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "VTR",
        740260,
        &["ventasreit.com", "www.ventasreit.com", "ir.ventasreit.com"],
        vec![(
            Metric::Ffo,
            Unit::PerShare,
            r"^(?:nareit )?ffo attributable to common stockholders per share$",
            1,
        )],
        None,
    ))?;
    company.debt = Some(debt);
    Ok(company)
}

fn debt(html: &scraper::Html, as_of: chrono::NaiveDate) -> anyhow::Result<crate::debt::Debt> {
    crate::debt::tables(html, as_of, &[crate::debt::Rule::new(3, 1000)], None)
}
