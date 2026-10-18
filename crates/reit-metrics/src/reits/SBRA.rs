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
    Ok(company)
}
