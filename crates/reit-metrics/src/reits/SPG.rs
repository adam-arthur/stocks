use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "SPG",
        1063761,
        &["www.simon.com", "investors.simon.com", "ir.simon.com"],
        vec![(
            Metric::Ffo,
            Unit::PerShare,
            r"^(?:basic and )?diluted ffo per share$",
            1,
        )],
        None,
    ))?;
    company.tables = crate::table::joined_parentheses;
    company.official_url = |url| {
        url.host_str() == Some("www.sec.gov")
            && url.path().starts_with("/Archives/edgar/data/1022344/")
    };
    company.debt = Some(debt);
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
