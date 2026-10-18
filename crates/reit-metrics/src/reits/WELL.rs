use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "WELL",
        766704,
        &["welltower.com", "www.welltower.com"],
        vec![
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^(?:nareit ffo|funds from operations\s*[-–—]\s*nareit)$",
                1,
            ),
            (Metric::Ffo, Unit::PerShare, r"^ffo per diluted share$", 1),
            (
                Metric::Ffo,
                Unit::Total,
                r"^(?:nareit )?ffo attributable to common stockholders$",
                1000,
            ),
        ],
        None,
    ))?;
    company.rules[0].section = Some(regex::Regex::new(r"(?i)^per (?:diluted )?share data")?);
    Ok(company)
}
