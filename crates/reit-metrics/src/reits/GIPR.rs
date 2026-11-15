use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "GIPR",
        1651721,
        &[],
        vec![(
            Metric::Ffo,
            Unit::Total,
            r"^(?:ffo|funds from operations)(?: attributable to common stockholders)?$",
            1,
        )],
        None,
    ))?;
    company.filings = &["8-K", "8-K/A", "10-Q", "10-K"];
    company.derive = Some(|documents| crate::derive::fourth_quarters("GIPR", documents));
    company.tables = crate::table::joined_parentheses;
    company.rules.push(crate::companies::Rule {
        metric: Metric::Affo,
        unit: Unit::Total,
        label: regex::Regex::new(r"^adjusted funds from operations$")?,
        scale: Some(1),
        section: None,
    });
    company.sections.push(affo_per_share);
    company.affo_support = crate::AffoSupport::Reported;
    company.debt = Some(debt);
    company.ownership = Some(crate::ownership::Rules::default());
    Ok(company)
}

fn debt(html: &scraper::Html, as_of: chrono::NaiveDate) -> anyhow::Result<crate::debt::Debt> {
    crate::debt::tables(html, as_of, &[crate::debt::Rule::new(2, 1)], None)
}

fn affo_per_share(tables: &[crate::table::Table], source: &str) -> Result<Vec<crate::Observation>> {
    let values = crate::affo_per_share(
        company()?,
        tables,
        source,
        crate::companies::Rule {
            metric: Metric::Affo,
            unit: Unit::Total,
            label: regex::Regex::new(
                r"^total weighted average shares of common stock outstanding [-–—] (?:basic & )?diluted$",
            )?,
            scale: Some(1),
            section: None,
        },
    )?;
    Ok(values)
}
