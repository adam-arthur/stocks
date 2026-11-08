use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "EQIX",
        1101239,
        &["investor.equinix.com", "www.equinix.com"][..],
        vec![
            (
                Metric::Ffo,
                Unit::Total,
                r"^ffo(?: attributable to common (?:stock|share)holders)?$",
                0,
            ),
            (Metric::Ffo, Unit::PerShare, r"^diluted ffo per share$", 1),
        ],
        None,
    ))?;
    company.tables = |html| {
        crate::table::tables(
            &regex::Regex::new(r"(?i)\(in\s+(thousands|millions)")
                .unwrap()
                .replace_all(html, "(dollars in $1"),
        )
    };
    Ok(company)
}
