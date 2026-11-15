use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "KRC",
        1025996,
        &["www.kilroyrealty.com", "investors.kilroyrealty.com"],
        vec![
            (
                Metric::Affo,
                Unit::Total,
                r"^funds available for distribution$",
                1000,
            ),
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^ffo per common share/unit\s*[-–—]\s*diluted$",
                1,
            ),
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^funds from operations per common share(?:/unit)?\s*[-–—]\s*diluted$",
                1,
            ),
            (Metric::Ffo, Unit::Total, r"^funds from operations$", 1000),
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
                r"^weighted average common shares/units outstanding\s*[-–—]\s*diluted$",
            )?,
            scale: Some(1000),
            section: None,
        },
    )
}

fn debt(html: &scraper::Html, as_of: chrono::NaiveDate) -> anyhow::Result<crate::debt::Debt> {
    crate::debt::tables(html, as_of, &[crate::debt::Rule::new(0, 1000)], None)
}
