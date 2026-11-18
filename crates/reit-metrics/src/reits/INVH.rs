use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "INVH",
        1687229,
        &["www.invitationhomes.com", "ir.invitationhomes.com"],
        vec![(
            Metric::Ffo,
            Unit::PerShare,
            r"^ffo per share\s*[-–—]\s*diluted$",
            1,
        )],
        None,
    ))?;
    company.tables = |html| {
        let mut tables = crate::table::tables(html);
        for table in &mut tables {
            if !table.rows.iter().flatten().any(|cell| {
                cell.text.to_lowercase().starts_with("affo reconciliation")
                    || cell
                        .text
                        .to_lowercase()
                        .starts_with("recurring capital expenditures")
            }) {
                table.rows.retain(|row| {
                    !row.iter()
                        .find(|cell| !cell.text.is_empty())
                        .is_some_and(|cell| {
                            cell.text
                                .split('(')
                                .next()
                                .is_some_and(|label| label.trim().eq_ignore_ascii_case("AFFO"))
                        })
                });
            }
        }
        tables
    };
    company.rules.extend([
        crate::companies::Rule {
            metric: Metric::Affo,
            unit: Unit::PerShare,
            label: regex::Regex::new(r"(?i)^affo per share\s*[-–—]\s*diluted$")?,
            scale: Some(1),
            section: None,
        },
        crate::companies::Rule {
            metric: Metric::Affo,
            unit: Unit::Total,
            label: regex::Regex::new(r"(?i)^affo$")?,
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
        &[crate::debt::Rule::new(usize::MAX, 1000)],
        None,
    )
}
