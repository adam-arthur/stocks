use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "WHLR",
        1527541,
        &["whlr.us", "www.whlr.us", "ir.whlr.us"],
        vec![(
            Metric::Ffo,
            Unit::PerShare,
            r"^ffo per common share(?: and (?:common|op) units?)?$",
            1,
        )],
        None,
    ))?;
    company.tables = tables;
    company.debt = Some(debt);
    Ok(company)
}

pub(crate) fn tables(html: &str) -> Vec<crate::table::Table> {
    let mut tables = crate::table::tables(html);
    for row in tables.iter_mut().flat_map(|table| &mut table.rows) {
        for index in 1..row.len() {
            if row[index].text == ")" && row[index - 1].text.starts_with('(') {
                row[index - 1].text.push(')');
                row[index].text.clear();
            }
        }
    }
    tables
}

fn debt(html: &scraper::Html, as_of: chrono::NaiveDate) -> anyhow::Result<crate::debt::Debt> {
    crate::debt::tables(html, as_of, &[crate::debt::Rule::new(0, 1000)], None)
}
