use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "CDP",
        860546,
        &[
            "www.copt.com",
            "investors.copt.com",
            "www.coptdefense.com",
            "investors.coptdefense.com",
        ],
        vec![(
            Metric::Ffo,
            Unit::PerShare,
            r"^diluted ffo per share(?: - nareit)?$",
            1,
        )],
        None,
    ))?;
    company.tables = crate::table::joined_parentheses;
    Ok(company)
}
