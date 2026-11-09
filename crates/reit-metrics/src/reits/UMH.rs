use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "UMH",
        752642,
        &["umh.reit", "www.umh.reit", "ir.umh.reit"],
        vec![(
            Metric::Ffo,
            Unit::PerShare,
            r"^ffo (?:per share\s*[-–]\s*(?:basic and )?diluted|per diluted common share)$",
            1,
        )],
        None,
    ))?;
    company.tables = crate::table::joined_parentheses;
    company.debt = Some(debt);
    Ok(company)
}

fn debt(html: &scraper::Html, as_of: chrono::NaiveDate) -> anyhow::Result<crate::debt::Debt> {
    crate::debt::tables(
        html,
        as_of,
        &([
            "SCHEDULE OF AGGREGATE PRINCIPAL PAYMENTS OF ALL LOANS PAYABLE",
            "SCHEDULE OF AGGREGATE PRINCIPAL PAYMENTS OF ALL MORTGAGES PAYABLE",
        ]
        .into_iter()
        .map(|intro| crate::debt::Rule {
            intro,
            ..crate::debt::Rule::new(0, 1000)
        })
        .collect::<Vec<_>>()),
        None,
    )
}
