use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "LAND",
        1495240,
        &[],
        vec![
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^(?:(?:diluted )?ffo per weighted[- ]average (?:common|total(?: common)?) share|ffo available to common stockholders(?: and (?:non-controlling op unitholders|op unit ?holders))?\s*[-–—]\s*diluted)$",
                1,
            ),
            (
                Metric::Ffo,
                Unit::Total,
                r"^ffo available to common stockholders(?: and (?:non-controlling op unitholders|op unit ?holders))?$",
                0,
            ),
        ],
        None,
    ))?;
    company.tables = crate::table::joined_parentheses;
    company.debt = Some(debt);
    Ok(company)
}

fn debt(html: &scraper::Html, as_of: chrono::NaiveDate) -> anyhow::Result<crate::debt::Debt> {
    let debt = crate::debt::tables(html, as_of, &[crate::debt::Rule::new(0, 1000)], None)?;
    let payments = crate::debt::instrument_payments(
        &crate::debt::rows(html, as_of, "Undrawn Commitment")?,
        (r"^MetLife Lines of Credit$", 3, 2),
    )?;
    anyhow::ensure!(
        crate::debt::scalar(html, "us-gaap:LineOfCredit", as_of)?
            == Some(i64::try_from(
                payments.iter().map(|(_, value)| value).sum::<u64>()
            )?),
        "LAND credit facility balance does not reconcile"
    );
    crate::debt::add_payments(debt, &payments)
}
