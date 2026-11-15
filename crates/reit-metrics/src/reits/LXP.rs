use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;
pub fn company() -> Result<Company> {
    let mut company = build((
        "LXP",
        910108,
        &["ir.lxp.com"],
        vec![
            (Metric::Ffo, Unit::PerShare, r"^ffo$", 1),
            (
                Metric::Affo,
                Unit::Total,
                r"^company funds available for distribution$",
                1000,
            ),
            (
                Metric::Ffo,
                Unit::Total,
                r"^ffo available to (?:all equityholders|common shareholders and unitholders) - diluted$",
                1000,
            ),
        ],
        None,
    ))?;
    company.rules[0].section = Some(regex::Regex::new(r"(?i)^diluted:?$")?);
    company.affo_support = crate::AffoSupport::Reported;
    company.sections.push(affo_per_share);
    company.tables = crate::table::joined_parentheses;
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
                r"^weighted-average common shares outstanding - diluted ffo$",
            )?,
            scale: Some(1),
            section: None,
        },
    )
}

fn debt(html: &scraper::Html, as_of: chrono::NaiveDate) -> anyhow::Result<crate::debt::Debt> {
    crate::debt::tables(html, as_of, &[crate::debt::Rule::new(0, 1000)], None)
}
