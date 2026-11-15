use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "ARE",
        1035443,
        &["www.are.com", "investor.are.com"],
        vec![
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^(?:ffo|funds from operations) per share attributable to alexandria['’]s common stockholders [–—-] diluted$",
                1,
            ),
            (
                Metric::Ffo,
                Unit::Total,
                r"^(?:ffo|funds from operations) attributable to alexandria['’]s common stockholders [–—-] diluted$",
                1000,
            ),
        ],
        Some("per share attributable to alexandria"),
    ))?;
    company.rules.extend([
        crate::companies::Rule {
            metric: Metric::Affo,
            unit: Unit::PerShare,
            label: regex::Regex::new(r"^affo per share attributable to alexandria['’]s common stockholders [–—-] diluted$")?,
            scale: Some(1),
            section: None,
        },
        crate::companies::Rule {
            metric: Metric::Affo,
            unit: Unit::Total,
            label: regex::Regex::new(r"^affo attributable to alexandria['’]s common stockholders [–—-] diluted$")?,
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
            row: Some(("Unsecured debt weighted-average", 0)),
            ..crate::debt::Rule::new(0, 1000)
        }],
        None,
    )
}
