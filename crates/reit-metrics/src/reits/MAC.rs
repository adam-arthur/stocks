use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "MAC",
        912242,
        &["investing.macerich.com", "www.macerich.com"],
        vec![
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^ffo per share\s*[-–—]\s*(?:basic and )?diluted$",
                1,
            ),
            (
                Metric::Ffo,
                Unit::Total,
                r"^ffo(?: attributable to common stockholders and unit holders)?\s*[-–—]\s*basic and diluted$",
                1000,
            ),
        ],
        None,
    ))?;
    company.affo_support = crate::AffoSupport::Unavailable;
    company.tables = |html| {
        let mut tables = crate::table::joined_parentheses(html);
        for table in &mut tables {
            if table.rows.iter().flatten().any(|c| {
                c.text
                    .to_lowercase()
                    .contains("weighted average shares outstanding")
            }) {
                for label in table
                    .rows
                    .iter_mut()
                    .filter_map(|row| row.iter_mut().find(|c| !c.text.is_empty()))
                {
                    if label.text.to_lowercase().starts_with("ffo per share") {
                        label.text = format!("Reconciled {}", label.text);
                    }
                }
            }
        }
        tables
    };
    company.rules.insert(
        0,
        crate::companies::Rule {
            metric: Metric::Ffo,
            unit: Unit::PerShare,
            label: regex::Regex::new(
                r"^reconciled ffo per share\s*[-–—]\s*(?:basic and )?diluted$",
            )?,
            scale: Some(1),
            section: None,
        },
    );
    company.debt = Some(debt);
    Ok(company)
}

fn debt(html: &scraper::Html, as_of: chrono::NaiveDate) -> anyhow::Result<crate::debt::Debt> {
    crate::debt::tables(html, as_of, &[crate::debt::Rule::new(0, 1000)], None)
}
