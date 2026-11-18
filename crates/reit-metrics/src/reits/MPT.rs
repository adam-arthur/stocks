use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "MPT",
        1287865,
        &[
            "ir.medicalpropertiestrust.com",
            "www.medicalpropertiestrust.com",
        ],
        vec![
            (Metric::Ffo, Unit::PerShare, r"^funds from operations$", 1),
            (Metric::Ffo, Unit::Total, r"^funds from operations$", 1000),
            (
                Metric::Affo,
                Unit::PerShare,
                r"^adjusted funds from operations$",
                1,
            ),
            (
                Metric::Affo,
                Unit::Total,
                r"^adjusted funds from operations$",
                1000,
            ),
        ],
        None,
    ))?;
    company.tables = |html| {
        let mut tables = crate::table::joined_parentheses(html);
        for table in &mut tables {
            table
                .rows
                .retain(|row| !row.iter().any(|cell| cell.text.eq_ignore_ascii_case("to")));
        }
        tables
    };
    company.rules[0].section = Some(regex::Regex::new(
        r"(?i)^per (?:diluted |common )?share data:?",
    )?);
    company.rules[1].section = Some(regex::Regex::new(r"(?i)^ffo information:?")?);
    company.affo_support = crate::AffoSupport::Reported;
    company.rules[2].section = company.rules[0].section.clone();
    company.rules[3].section = company.rules[1].section.clone();
    company.debt = Some(debt);
    company.ownership = Some(crate::ownership::Rules::default());
    Ok(company)
}

fn debt(html: &scraper::Html, as_of: chrono::NaiveDate) -> anyhow::Result<crate::debt::Debt> {
    crate::debt::tables(html, as_of, &[crate::debt::Rule::new(0, 1000)], None)
}
