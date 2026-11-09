use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "EGP",
        49600,
        &["eastgroup.net", "www.eastgroup.net", "ir.eastgroup.net"],
        vec![(
            Metric::Ffo,
            Unit::PerShare,
            r"^(?:ffo|funds from operations) attributable to common stockholders$",
            1,
        )],
        None,
    ))?;
    company.tables = crate::table::joined_parentheses;
    company.rules[0].section = Some(regex::Regex::new(r"(?i)^diluted per common share data")?);
    company.debt = Some(debt);
    Ok(company)
}

fn debt(html: &scraper::Html, as_of: chrono::NaiveDate) -> anyhow::Result<crate::debt::Debt> {
    crate::debt::tables(
        html,
        as_of,
        &[
            crate::debt::Rule {
                anchor: "",
                intro: "principal payments due and weighted average interest rates",
                row: Some(("Unsecured bank credit facilities", 0)),
                ..crate::debt::Rule::new(0, 1000)
            },
            crate::debt::Rule {
                anchor: "",
                intro: "principal payments due and weighted average interest rates",
                row: Some(("Unsecured debt", 0)),
                ..crate::debt::Rule::new(0, 1000)
            },
        ],
        None,
    )
}
