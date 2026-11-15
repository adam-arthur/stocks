use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "SLG",
        1040971,
        &["slgreen.com", "www.slgreen.com", "slgreen.gcs-web.com"],
        vec![
            (Metric::Ffo, Unit::PerShare, r"^diluted ffo per share$", 1),
            (
                Metric::Affo,
                Unit::Total,
                r"^reported funds available for distribution$",
                1000,
            ),
            (
                Metric::Affo,
                Unit::PerShare,
                r"^reported funds available for distribution to common stockholders\s*[-–—]\s*diluted$",
                1,
            ),
        ],
        None,
    ))?;
    company.tables = tables;
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
                r"^(?:weighted average common shares and units outstanding\s*[-–—]\s*diluted|diluted weighted average shares and units outstanding)$",
            )?,
            scale: Some(1000),
            section: None,
        },
    )
}

fn tables(html: &str) -> Vec<crate::table::Table> {
    crate::table::joined_parentheses(html)
        .into_iter()
        .map(|mut table| {
            for cell in table.rows.iter_mut().flatten() {
                if cell
                    .text
                    .to_lowercase()
                    .starts_with("ffo per share (diluted)")
                {
                    cell.text = "Diluted FFO per share".into();
                }
            }
            table
        })
        .collect()
}

fn debt(html: &scraper::Html, as_of: chrono::NaiveDate) -> anyhow::Result<crate::debt::Debt> {
    crate::debt::tables(html, as_of, &[crate::debt::Rule::new(4, 1000)], None)
}
