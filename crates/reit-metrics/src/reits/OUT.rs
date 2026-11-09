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
    company.debt = Some(debt);
    company.ownership = Some(crate::ownership::Rules::default());
    Ok(company)
}

fn debt(html: &scraper::Html, as_of: chrono::NaiveDate) -> anyhow::Result<crate::debt::Debt> {
    crate::debt::tables(
        html,
        as_of,
        &[crate::debt::Rule {
            row: Some(("Long-term debt", 0)),
            ..crate::debt::Rule::new(0, 1000000)
        }],
        None,
    )
}
