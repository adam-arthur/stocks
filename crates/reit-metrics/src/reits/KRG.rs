use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "KRG",
        1286043,
        &["www.kiterealty.com", "investor.kiterealty.com"],
        vec![
            (
                Metric::Affo,
                Unit::Total,
                r"^total recurring affo of the operating partnership$",
                1000,
            ),
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^ffo, as defined by nareit, per share of the operating partnership\s*[-–—]\s*diluted$",
                1,
            ),
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^ffo per share of the operating partnership\s*[-–—]\s*diluted$",
                1,
            ),
            (
                Metric::Ffo,
                Unit::Total,
                r"^(?:ffo|funds from operations) attributable to (?:kite realty group trust )?common shareholders(?: 1)?$",
                1000,
            ),
        ],
        None,
    ))?;
    company.affo_support = crate::AffoSupport::Reported;
    company.sections.push(affo_per_share);
    company.debt = Some(debt);
    Ok(company)
}

fn affo_per_share(tables: &[crate::table::Table], source: &str) -> Result<Vec<crate::Observation>> {
    crate::affo_per_share(
        company()?,
        tables,
        source,
        crate::companies::Rule {
            metric: Metric::Affo,
            unit: Unit::Total,
            label: regex::Regex::new(
                r"^weighted average common shares and units outstanding\s*[-–—]\s*diluted$",
            )?,
            scale: Some(1),
            section: None,
        },
    )
}

fn debt(html: &scraper::Html, as_of: chrono::NaiveDate) -> anyhow::Result<crate::debt::Debt> {
    crate::debt::tables(
        html,
        as_of,
        &[crate::debt::Rule {
            anchor: "",
            intro: "scheduled maturities and principal amortization of the Company",
            ..crate::debt::Rule::new(3, 1000)
        }],
        None,
    )
}
