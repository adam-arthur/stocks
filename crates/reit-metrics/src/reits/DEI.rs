use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "DEI",
        1364250,
        &[
            "douglasemmett.com",
            "www.douglasemmett.com",
            "ir.douglasemmett.com",
        ],
        vec![
            (Metric::Affo, Unit::Total, r#"^affo$"#, 1000),
            (Metric::Affo, Unit::Total, r"^affo in millions$", 1_000_000),
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^ffo per (?:fully diluted share|share\s*[-–—]\s*fully diluted)$",
                1,
            ),
        ],
        None,
    ))?;
    company.tables = |html| {
        let mut tables = crate::table::joined_parentheses(html);
        for table in &mut tables {
            if table
                .rows
                .iter()
                .take(8)
                .flatten()
                .any(|cell| cell.text.to_ascii_lowercase().contains("in millions"))
            {
                for cell in table
                    .rows
                    .iter_mut()
                    .flatten()
                    .filter(|cell| cell.text.eq_ignore_ascii_case("AFFO"))
                {
                    cell.text = "AFFO in millions".into();
                }
            }
        }
        tables
    };
    company.affo_support = crate::AffoSupport::Reported;
    company.debt = Some(debt);
    Ok(company)
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
