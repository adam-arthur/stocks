use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "BNL",
        1424182,
        &[
            "investors.broadstone.com",
            "broadstone.com",
            "www.broadstone.com",
        ],
        vec![
            (
                Metric::Affo,
                Unit::PerShare,
                r#"^affo per (?:diluted )?share(?:, (?:basic and )?diluted)?(?:\s+\d+)?$"#,
                1,
            ),
            (Metric::Affo, Unit::Total, r#"^affo$"#, 1000),
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^ffo per (?:diluted )?share(?:, (?:basic and )?diluted)?(?:\s+\d+)?$",
                1,
            ),
            (Metric::Ffo, Unit::Total, r"^ffo$", 1000),
        ],
        None,
    ))?;
    company.affo_support = crate::AffoSupport::Reported;
    company.tables = |html| {
        crate::table::tables(html)
            .into_iter()
            .filter(|table| {
                !table
                    .rows
                    .iter()
                    .take(8)
                    .flatten()
                    .any(|cell| cell.text.eq_ignore_ascii_case("Revenues"))
            })
            .collect()
    };
    company.debt = Some(debt);
    Ok(company)
}

fn debt(html: &scraper::Html, as_of: chrono::NaiveDate) -> anyhow::Result<crate::debt::Debt> {
    crate::debt::tables(html, as_of, &[crate::debt::Rule::new(0, 1000)], None)
}
