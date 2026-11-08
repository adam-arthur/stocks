use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "NTST",
        1798100,
        &[
            "investors.netstreit.com",
            "www.netstreit.com",
            "netstreit.com",
        ],
        vec![
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^ffo per common share, diluted$",
                1,
            ),
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^funds from operations per diluted share$",
                1,
            ),
            (Metric::Ffo, Unit::Total, r"^ffo$", 1000),
        ],
        None,
    ))?;
    company.tables = crate::table::joined_parentheses;
    Ok(company)
}
