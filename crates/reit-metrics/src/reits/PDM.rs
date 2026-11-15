use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "PDM",
        1042776,
        &[
            "piedmontreit.com",
            "www.piedmontreit.com",
            "investor.piedmontreit.com",
            "piedmontrealtytrust.com",
            "www.piedmontrealtytrust.com",
        ],
        vec![
            (
                Metric::Affo,
                Unit::PerShare,
                r"^(?:adjusted funds from operations per share(?: diluted)?|affo per share - diluted)$",
                1,
            ),
            (
                Metric::Affo,
                Unit::Total,
                r"^adjusted funds from operations(?: applicable to common stock)?$",
                1000,
            ),
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^(?:nareit )?funds from operations per share(?: diluted)?$",
                1,
            ),
        ],
        None,
    ))?;
    company.tables = crate::table::joined_parentheses;
    company.sections.push(affo_per_share);
    company.affo_support = crate::AffoSupport::Reported;
    company.debt = Some(debt);
    Ok(company)
}

fn debt(html: &scraper::Html, as_of: chrono::NaiveDate) -> anyhow::Result<crate::debt::Debt> {
    crate::debt::tables(html, as_of, &[crate::debt::Rule::new(0, 1000)], None)
}

fn affo_per_share(tables: &[crate::table::Table], source: &str) -> Result<Vec<crate::Observation>> {
    crate::affo_per_share(
        company()?,
        tables,
        source,
        crate::companies::Rule {
            metric: Metric::Affo,
            unit: Unit::Total,
            label: regex::Regex::new(r"^weighted average common shares outstanding - diluted$")?,
            scale: Some(1000),
            section: None,
        },
    )
}
