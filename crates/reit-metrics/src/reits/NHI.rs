use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "NHI",
        877860,
        &["investors.nhireit.com", "www.nhireit.com"],
        vec![
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^nareit ffo attributable to common stockholders per share$",
                1,
            ),
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^nareit ffo per diluted share$",
                1,
            ),
            (
                Metric::Ffo,
                Unit::Total,
                r"^nareit ffo(?: attributable to common stockholders)?$",
                1000,
            ),
            (
                Metric::Affo,
                Unit::Total,
                r"^normalized affo(?: attributable to common stockholders)?$",
                1000,
            ),
            (
                Metric::Affo,
                Unit::PerShare,
                r"^normalized affo (?:attributable to common stockholders per share|per common share)$",
                1,
            ),
            (
                Metric::Affo,
                Unit::Total,
                r"^normalized fad(?: attributable to common stockholders)?$",
                1000,
            ),
        ],
        None,
    ))?;
    company.tables = crate::table::joined_parentheses;
    company.rules[0].section = Some(regex::Regex::new(r"(?i)^diluted:?")?);
    company.rules[4].section = company.rules[0].section.clone();
    company.affo_support = crate::AffoSupport::Reported;
    company.sections.push(affo_per_share);
    company.debt = Some(debt);
    company.ownership = Some(crate::ownership::Rules::default());
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
            label: regex::Regex::new(r"^weighted average common shares outstanding$")?,
            scale: Some(1),
            section: Some(regex::Regex::new(r"(?i)^diluted:?")?),
        },
    )
}

fn debt(html: &scraper::Html, as_of: chrono::NaiveDate) -> anyhow::Result<crate::debt::Debt> {
    crate::debt::tables(
        html,
        as_of,
        &[crate::debt::Rule {
            anchor: "us-gaap:LongTermDebtMaturitiesRepaymentsOfPrincipalInNextRollingTwelveMonths",
            ..crate::debt::Rule::new(0, 1000)
        }],
        None,
    )
}
