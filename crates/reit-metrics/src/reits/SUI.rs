use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "SUI",
        912593,
        &[
            "suncommunities.com",
            "www.suncommunities.com",
            "ir.suncommunities.com",
        ],
        vec![(
            Metric::Ffo,
            Unit::PerShare,
            r"^ffo(?: attributable to sun communities, inc\. common stockholders and dilutive convertible securities)? per share(?: - fully diluted)?$",
            1,
        )],
        None,
    ))?;
    company.tables = tables;
    Ok(company)
}

pub(crate) fn tables(html: &str) -> Vec<crate::table::Table> {
    crate::table::joined_parentheses(html)
        .into_iter()
        .filter(|table| {
            !table.rows.iter().take(5).any(|row| {
                row.iter().any(|cell| cell.text == "Reported")
                    && row.iter().any(|cell| cell.text == "Adjusted")
            })
        })
        .collect()
}
