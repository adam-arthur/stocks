use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "NLOP",
        1952976,
        &["nloproperties.com", "www.nloproperties.com"],
        vec![
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^ffo \(as defined by nareit\) attributable to nlop per diluted share$",
                1,
            ),
            (
                Metric::Ffo,
                Unit::Total,
                r"^ffo \(as defined by nareit\) attributable to nlop$",
                1000,
            ),
        ],
        None,
    ))?;
    company.tables = crate::table::joined_parentheses;
    company.additional_items = Some(|items| items.split(',').any(|item| item.trim() == "7.01"));
    Ok(company)
}
