use crate::companies::{Company, build};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "WY",
        106535,
        &[
            "weyerhaeuser.com",
            "www.weyerhaeuser.com",
            "investor.weyerhaeuser.com",
        ],
        vec![],
        None,
    ))?;
    company.debt = Some(debt);
    Ok(company)
}

fn debt(html: &scraper::Html, as_of: chrono::NaiveDate) -> anyhow::Result<crate::debt::Debt> {
    crate::debt::tables(html, as_of, &[crate::debt::Rule::new(0, 1000000)], None)
}
