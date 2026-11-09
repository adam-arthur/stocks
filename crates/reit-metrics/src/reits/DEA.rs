use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "DEA",
        1622194,
        &[],
        vec![
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^ffo,? per share\s*[-–—]\s*(?:weighted average )?fully diluted basis$",
                1,
            ),
            (
                Metric::Ffo,
                Unit::Total,
                r"^(?:funds from operations|ffo)$",
                1000,
            ),
        ],
        None,
    ))?;
    company.tables = crate::table::joined_parentheses;
    company.debt = Some(debt);
    Ok(company)
}

fn debt(html: &scraper::Html, as_of: chrono::NaiveDate) -> anyhow::Result<crate::debt::Debt> {
    use anyhow::Context;
    // Mortgage balances and final dates omit intervening amortization; do not allocate the entire balance to maturity.
    crate::debt::tables(html, as_of, &[crate::debt::Rule::new(0, 1000)], None)
        .context("DEA needs scheduled mortgage principal payments by year")
}
