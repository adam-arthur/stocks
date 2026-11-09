use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "PK",
        1617406,
        &[
            "pkhotelsandresorts.com",
            "www.pkhotelsandresorts.com",
            "investors.pkhotelsandresorts.com",
        ],
        vec![(
            Metric::Ffo,
            Unit::PerShare,
            r"^(?:nareit )?ffo per share\s*[-–—]\s*diluted$",
            1,
        )],
        None,
    ))?;
    company.tables = crate::table::joined_parentheses;
    company.debt = Some(debt);
    Ok(company)
}

fn debt(html: &scraper::Html, as_of: chrono::NaiveDate) -> anyhow::Result<crate::debt::Debt> {
    crate::debt::tables(html, as_of, &[crate::debt::Rule::new(0, 1000000)], None)
}
