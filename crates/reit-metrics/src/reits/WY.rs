use crate::companies::{Company, build};
use crate::{Metric, Unit};
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
        vec![(Metric::Affo, Unit::Total, r"^adjusted fad$", 1000000)],
        None,
    ))?;
    company.affo_support = crate::AffoSupport::Reported;
    company.sections.push(affo_per_share);
    company.tables = crate::table::joined_parentheses;
    company.debt = Some(debt);
    Ok(company)
}

fn debt(html: &scraper::Html, as_of: chrono::NaiveDate) -> anyhow::Result<crate::debt::Debt> {
    crate::debt::tables(html, as_of, &[crate::debt::Rule::new(0, 1000000)], None)
}

fn affo_per_share(tables: &[crate::table::Table], source: &str) -> Result<Vec<crate::Observation>> {
    crate::affo_per_share(
        company()?,
        tables,
        source,
        crate::companies::Rule {
            metric: Metric::Affo,
            unit: Unit::Total,
            label: regex::Regex::new(r"^weighted average shares outstanding, diluted$")?,
            scale: Some(1000000),
            section: None,
        },
    )
}
