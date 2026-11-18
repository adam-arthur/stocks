use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "CBL",
        910612,
        &["www.cblproperties.com", "ir.cblproperties.com"],
        vec![
            (Metric::Ffo, Unit::PerShare, r"^ffo per diluted share$", 1),
            (
                Metric::Ffo,
                Unit::Total,
                r"^ffo allocable to common shareholders$",
                1000,
            ),
            (
                Metric::Ffo,
                Unit::Total,
                r"^ffo allocable to operating partnership common unitholders$",
                1000,
            ),
        ],
        None,
    ))?;
    company.affo_support = crate::AffoSupport::Unavailable;
    company.tables = |html| {
        crate::table::joined_parentheses(html)
            .into_iter()
            .filter(|table| {
                !table.rows.iter().flatten().any(|cell| {
                    cell.text
                        .eq_ignore_ascii_case("Add amounts per share included in FFO:")
                })
            })
            .collect()
    };
    company.debt = Some(debt);
    company.ownership = Some(crate::ownership::Rules::default());
    Ok(company)
}

fn debt(html: &scraper::Html, as_of: chrono::NaiveDate) -> anyhow::Result<crate::debt::Debt> {
    crate::debt::tables(html, as_of, &[crate::debt::Rule::new(0, 1000)], None)
}
