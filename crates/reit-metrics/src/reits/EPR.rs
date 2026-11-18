use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "EPR",
        1045450,
        &["investors.eprkc.com"],
        vec![
            (
                Metric::Affo,
                Unit::PerShare,
                r#"^affo per (?:diluted common share|common share: diluted)$"#,
                1,
            ),
            (
                Metric::Affo,
                Unit::Total,
                r#"^affo available to common shareholders of epr properties$"#,
                1000,
            ),
            (
                Metric::Ffo,
                Unit::PerShare,
                r#"^ffo per (?:diluted common share|common share: diluted)$"#,
                1,
            ),
            (
                Metric::Ffo,
                Unit::Total,
                r#"^ffo available to common shareholders of epr properties$"#,
                1000,
            ),
        ],
        None,
    ))?;
    company.tables = |html| {
        let mut tables = crate::table::joined_parentheses(html);
        for table in &mut tables {
            let mut adjusted = false;
            table.rows.retain(|row| {
                let label = row
                    .iter()
                    .find(|cell| !cell.text.is_empty())
                    .map(|cell| cell.text.to_lowercase())
                    .unwrap_or_default();
                if label.starts_with("ffo:") || label.starts_with("ffoaa:") {
                    adjusted = label.starts_with("ffoaa:");
                }
                !(adjusted && label == "ffo available to common shareholders of epr properties")
            });
        }
        tables
    };
    company.affo_support = crate::AffoSupport::Reported;
    company.debt = Some(debt);
    Ok(company)
}

fn debt(html: &scraper::Html, as_of: chrono::NaiveDate) -> anyhow::Result<crate::debt::Debt> {
    use anyhow::Context;
    // Quarterly financing changes alone do not establish maturities for all outstanding notes and mortgage debt.
    crate::debt::tables(html, as_of, &[crate::debt::Rule::new(0, 1000)], None)
        .context("EPR needs a complete current principal maturity breakdown")
}
