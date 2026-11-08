use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "PECO",
        1476204,
        &[
            "phillipsedison.com",
            "www.phillipsedison.com",
            "investors.phillipsedison.com",
        ],
        vec![(
            Metric::Ffo,
            Unit::PerShare,
            r"^(?:nareit )?ffo(?: attributable to stockholders and (?:convertible noncontrolling interests|op unit holders))? per share\s*[-–—]\s*(?:basic and )?diluted$",
            1,
        )],
        None,
    ))?;
    company.filings = &["8-K", "8-K/A", "10-Q", "10-Q/A", "10-K", "10-K/A"];
    company.tables = crate::table::joined_parentheses;
    Ok(company)
}
