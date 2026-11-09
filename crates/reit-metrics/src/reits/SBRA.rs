use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "SBRA",
        1492298,
        &[
            "sabrahealth.com",
            "www.sabrahealth.com",
            "ir.sabrahealth.com",
        ],
        vec![(Metric::Ffo, Unit::PerShare, r"^ffo$", 1)],
        None,
    ))?;
    company.rules[0].section = Some(regex::Regex::new(
        r"(?i)^amounts per diluted common share(?: attributable to (?:common stockholders|sabra health care reit, inc\.))?:",
    )?);
    company.tables = crate::table::joined_parentheses;
    company.debt = Some(debt);
    Ok(company)
}

fn debt(html: &scraper::Html, as_of: chrono::NaiveDate) -> anyhow::Result<crate::debt::Debt> {
    crate::debt::tables(html, as_of, &[crate::debt::Rule::new(4, 1000)], None)
}
