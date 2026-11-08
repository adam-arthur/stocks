use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "LAMR",
        1090425,
        &[],
        vec![(Metric::Ffo, Unit::Total, r"^funds from operations$", 1000)],
        None,
    ))?;
    company.tables = crate::table::joined_parentheses;
    company.official_url = |url| {
        url.host_str() == Some("www.sec.gov")
            && url.path().starts_with("/Archives/edgar/data/899045/")
    };
    Ok(company)
}
