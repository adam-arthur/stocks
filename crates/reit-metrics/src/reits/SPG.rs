use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "SPG",
        1063761,
        &["www.simon.com", "investors.simon.com", "ir.simon.com"],
        vec![
            (
                Metric::Affo,
                Unit::Total,
                r"^funds available for distribution$",
                1000,
            ),
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^(?:basic and )?diluted ffo per share$",
                1,
            ),
        ],
        None,
    ))?;
    company.tables = crate::table::joined_parentheses;
    company.continuation = Some("details for per share calculations");
    company.official_url = |url| {
        url.host_str() == Some("www.sec.gov")
            && url.path().starts_with("/Archives/edgar/data/1022344/")
    };
    company.debt = Some(debt);
    company.affo_support = crate::AffoSupport::Reported;
    company.sections.push(affo_per_share);
    Ok(company)
}

fn debt(html: &scraper::Html, as_of: chrono::NaiveDate) -> anyhow::Result<crate::debt::Debt> {
    crate::debt::tables(
        html,
        as_of,
        &[crate::debt::Rule {
            heading: "Other Debt Obligations and Other",
            ..crate::debt::Rule::new(0, 1000)
        }],
        None,
    )
}

fn affo_per_share(tables: &[crate::table::Table], source: &str) -> Result<Vec<crate::Observation>> {
    crate::affo_per_share(
        company()?,
        tables,
        source,
        crate::companies::Rule {
            metric: Metric::Affo,
            unit: Unit::Total,
            label: regex::Regex::new(
                r"^basic and diluted weighted average shares and units outstanding$",
            )?,
            scale: Some(1000),
            section: None,
        },
    )
}
