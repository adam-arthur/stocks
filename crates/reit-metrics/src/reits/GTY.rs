use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "GTY",
        1052752,
        &[],
        vec![
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^(?:ffo|funds from operations) per (?:diluted )?share$",
                1,
            ),
            (Metric::Ffo, Unit::Total, r"^funds from operations$", 1000),
        ],
        None,
    ))?;
    company.tables = |html| {
        let mut tables = crate::table::joined_parentheses(html);
        for table in &mut tables {
            let mut basic = false;
            table.rows.retain(|row| {
                let label = row
                    .iter()
                    .find(|cell| !cell.text.is_empty())
                    .map(|cell| cell.text.to_lowercase())
                    .unwrap_or_default();
                if label.starts_with("basic per share amounts") {
                    basic = true;
                }
                if label.starts_with("diluted per share amounts") {
                    basic = false;
                }
                !basic || !label.ends_with("per share")
            });
        }
        tables
    };
    company.rules.extend([
        crate::companies::Rule {
            metric: Metric::Affo,
            unit: Unit::PerShare,
            label: regex::Regex::new(
                r"(?i)^(?:affo|adjusted funds from operations) per (?:diluted )?share$",
            )?,
            scale: Some(1),
            section: None,
        },
        crate::companies::Rule {
            metric: Metric::Affo,
            unit: Unit::Total,
            label: regex::Regex::new(r"(?i)^adjusted funds from operations$")?,
            scale: Some(1000),
            section: None,
        },
    ]);
    company.affo_support = crate::AffoSupport::Reported;
    company.debt = Some(debt);
    Ok(company)
}

fn debt(html: &scraper::Html, as_of: chrono::NaiveDate) -> anyhow::Result<crate::debt::Debt> {
    crate::debt::tables(
        html,
        as_of,
        &[crate::debt::Rule {
            anchor: "us-gaap:LongTermDebtMaturitiesRepaymentsOfPrincipalInYearThree",
            ..crate::debt::Rule::new(2, 1000)
        }],
        None,
    )
}
