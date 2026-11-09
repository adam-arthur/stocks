use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "XHR",
        1616000,
        &[
            "xeniareit.com",
            "www.xeniareit.com",
            "investors.xeniareit.com",
        ],
        vec![(
            Metric::Ffo,
            Unit::Total,
            r"^ffo attributable to common stock and unit holders$",
            1000,
        )],
        None,
    ))?;
    company.debt = Some(debt);
    company.ownership = Some(crate::ownership::Rules::default());
    Ok(company)
}

fn debt(html: &scraper::Html, as_of: chrono::NaiveDate) -> anyhow::Result<crate::debt::Debt> {
    crate::debt::tables(html, as_of, &[crate::debt::Rule::new(0, 1000)], None)
}
