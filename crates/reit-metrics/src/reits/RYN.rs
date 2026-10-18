use crate::companies::{Company, build};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "RYN",
        52827,
        &["www.rayonier.com", "rayonier.com", "ir.rayonier.com"],
        vec![],
        None,
    ))?;
    company.tables = crate::table::joined_parentheses;
    Ok(company)
}
