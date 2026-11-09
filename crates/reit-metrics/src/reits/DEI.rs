use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "DEI",
        1364250,
        &[
            "douglasemmett.com",
            "www.douglasemmett.com",
            "ir.douglasemmett.com",
        ],
        vec![(
            Metric::Ffo,
            Unit::PerShare,
            r"^ffo per (?:fully diluted share|share\s*[-–—]\s*fully diluted)$",
            1,
        )],
        None,
    ))?;
    company.tables = crate::table::joined_parentheses;
    company.debt = Some(debt);
    Ok(company)
}

fn debt(html: &scraper::Html, as_of: chrono::NaiveDate) -> anyhow::Result<crate::debt::Debt> {
    crate::debt::tables(
        html,
        as_of,
        &[crate::debt::Rule {
            anchor: "us-gaap:LongTermDebtMaturitiesRepaymentsOfPrincipalInNextRollingTwelveMonths",
            ..crate::debt::Rule::new(0, 1000)
        }],
        None,
    )
}
