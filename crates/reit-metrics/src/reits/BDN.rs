use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "BDN",
        790816,
        &["investor.brandywinerealty.com", "www.brandywinerealty.com"],
        vec![
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^ffo per share\s*[-–—]\s*(?:basic/)?fully diluted$",
                1,
            ),
            (
                Metric::Ffo,
                Unit::Total,
                r"^funds from operations available to common share and unit holders$",
                1000,
            ),
        ],
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
            intro: "aggregate scheduled principal payments on",
            ..crate::debt::Rule::new(0, 1000)
        }],
        None,
    )
}
