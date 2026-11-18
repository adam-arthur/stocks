use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "EQIX",
        1101239,
        &["investor.equinix.com", "www.equinix.com"][..],
        vec![
            (
                Metric::Affo,
                Unit::PerShare,
                r#"^diluted affo per share$"#,
                1,
            ),
            (
                Metric::Affo,
                Unit::Total,
                r#"^affo(?: attributable to common (?:stock|share)holders)?$"#,
                0,
            ),
            (
                Metric::Ffo,
                Unit::Total,
                r"^ffo(?: attributable to common (?:stock|share)holders)?$",
                0,
            ),
            (Metric::Ffo, Unit::PerShare, r"^diluted ffo per share$", 1),
        ],
        None,
    ))?;
    company.tables = |html| {
        let mut tables = crate::table::tables(html);
        for table in &mut tables {
            if let Some(start) = table
                .rows
                .iter()
                .position(|row| row.iter().any(|cell| cell.text == "(1)"))
                && table.rows[..start].iter().flatten().any(|cell| {
                    cell.text
                        .to_ascii_lowercase()
                        .starts_with("diluted affo per share")
                })
            {
                table.rows.truncate(start);
            }
        }
        tables
    };
    company.affo_support = crate::AffoSupport::Reported;
    company.debt = Some(debt);
    company.ownership = Some(crate::ownership::Rules::default());
    Ok(company)
}

fn debt(html: &scraper::Html, as_of: chrono::NaiveDate) -> anyhow::Result<crate::debt::Debt> {
    crate::debt::tables(html, as_of, &[crate::debt::Rule::new(0, 1000000)], None)
}
