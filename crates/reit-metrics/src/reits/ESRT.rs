use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "ESRT",
        1541401,
        &[
            "investors.empirestaterealtytrust.com",
            "investors.esrtreit.com",
        ],
        vec![
            (Metric::Affo, Unit::Total, r"^core fad$", 1000),
            (Metric::Ffo, Unit::PerShare, r#"^ffo per share diluted$"#, 1),
            (
                Metric::Ffo,
                Unit::Total,
                r#"^ffo attributable to common stockholders and operating partnership units$"#,
                1000,
            ),
        ],
        Some("core ffo"),
    ))?;
    company.tables = crate::table::joined_parentheses;
    company.sections.push(affo_per_share);
    company.affo_support = crate::AffoSupport::Reported;
    company.affo_comparatives = true;
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
                r"^total weighted average shares and operating partnership units diluted$",
            )?,
            scale: Some(1000),
            section: Some(regex::Regex::new(
                r"(?i)^total weighted average shares and operating partnership units$",
            )?),
        },
    )
}

fn debt(html: &scraper::Html, as_of: chrono::NaiveDate) -> anyhow::Result<crate::debt::Debt> {
    crate::debt::tables(html, as_of, &[crate::debt::Rule::new(2, 1000)], None)
}
