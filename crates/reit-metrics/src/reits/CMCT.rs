use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "CMCT",
        908311,
        &[
            "www.creativemediacommunity.com",
            "investors.creativemediacommunity.com",
            "www.cimcommercial.com",
        ],
        vec![
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^ffo (?:attributable|available) to common stockholders per (?:share:?(?: -)? diluted|diluted share)$",
                1,
            ),
            (
                Metric::Ffo,
                Unit::Total,
                r"^ffo (?:(?:attributable|available) to common stockholders)$",
                1000,
            ),
            (Metric::Ffo, Unit::Total, r"^ffo$", 1000),
        ],
        None,
    ))?;
    company.filings = &["8-K", "8-K/A", "10-Q", "10-K"];
    company.tables = crate::table::joined_parentheses;
    Ok(company)
}
