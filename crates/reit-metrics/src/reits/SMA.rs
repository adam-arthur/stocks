use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "SMA",
        1585389,
        &[
            "smartstopselfstorage.com",
            "www.smartstopselfstorage.com",
            "investors.smartstopselfstorage.com",
        ],
        vec![
            (
                Metric::Ffo,
                Unit::Total,
                r"^ffo \(attributable to common stockholders\)$",
                0,
            ),
            (
                Metric::Ffo,
                Unit::Total,
                r"^ffo \(attributable to common stockholders and op unit holders\)$",
                0,
            ),
            (Metric::Ffo, Unit::Total, r"^ffo$", 0),
        ],
        None,
    ))?;
    company.filings = &["8-K", "8-K/A", "10-Q", "10-Q/A", "10-K", "10-K/A"];
    company.tables = tables;
    Ok(company)
}

fn tables(html: &str) -> Vec<crate::table::Table> {
    crate::table::joined_parentheses(html)
        .into_iter()
        .map(|mut table| {
            table.amount_scale = Some(table.amount_scale.unwrap_or(1));
            table
        })
        .collect()
}
