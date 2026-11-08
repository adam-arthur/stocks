use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "OUT",
        1579877,
        &[
            "outfront.com",
            "www.outfront.com",
            "outfrontmedia.com",
            "www.outfrontmedia.com",
            "investor.outfront.com",
            "investor.outfrontmedia.com",
        ],
        vec![(
            Metric::Ffo,
            Unit::Total,
            r"^(?:funds from operations|ffo attributable to outfront media inc\.)$",
            1000000,
        )],
        None,
    ))?;
    company.tables = crate::table::joined_parentheses;
    Ok(company)
}
