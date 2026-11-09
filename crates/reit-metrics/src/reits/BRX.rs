use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "BRX",
        1581068,
        &["www.brixmor.com", "investors.brixmor.com"],
        vec![(
            Metric::Ffo,
            Unit::PerShare,
            r"^nareit ffo per (?:diluted share|share/op unit - diluted)$",
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
