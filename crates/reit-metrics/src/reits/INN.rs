use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "INN",
        1497645,
        &["www.shpreit.com", "investor.shpreit.com"],
        vec![
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^ffo per diluted share and unit(?: \([\d, ]+\))?$",
                1,
            ),
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^ffo per (?:common share\s*/\s*common unit|share of common stock and common units)$",
                1,
            ),
            (
                Metric::Ffo,
                Unit::Total,
                r"^ffo applicable to (?:common shares and common units|common stock and common units)$",
                1000,
            ),
        ],
        None,
    ))?;
    company.tables = crate::table::joined_parentheses;
    company.additional_items = Some(|items| items.trim() == "9.01");
    Ok(company)
}
