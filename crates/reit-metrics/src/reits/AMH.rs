use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;
use regex::Regex;

pub fn company() -> Result<Company> {
    let mut company = build((
        "AMH",
        1562401,
        &["investors.amh.com", "www.amh.com"],
        vec![
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^ffo attributable to common share and unit holders$",
                1,
            ),
            (
                Metric::Ffo,
                Unit::Total,
                r"^ffo attributable to common share and unit holders$",
                1000,
            ),
        ],
        None,
    ))?;
    company.rules[0].section = Some(Regex::new(r"(?i)^per ffo share and unit:$")?);
    company.rules[1].section = Some(Regex::new(r"^$")?);
    Ok(company)
}
