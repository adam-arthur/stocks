use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;
pub fn company() -> Result<Company> {
    let mut company = build((
        "LTC",
        887905,
        &[],
        vec![
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^(?:nareit diluted ffo per common share|(?:nareit diluted|diluted(?: nareit)?) ffo attributable to common stockholders per share)$",
                1,
            ),
            (
                Metric::Ffo,
                Unit::Total,
                r"^(?:nareit )?(?:funds from operations|ffo) attributable to common stockholders$",
                1000,
            ),
        ],
        None,
    ))?;
    company.tables = crate::table::joined_parentheses;
    company.debt = Some(debt);
    company.ownership = Some(crate::ownership::Rules::default());
    Ok(company)
}

fn debt(html: &scraper::Html, as_of: chrono::NaiveDate) -> anyhow::Result<crate::debt::Debt> {
    crate::debt::tables(
        html,
        as_of,
        &[crate::debt::Rule {
            row: Some(("$", 1)),
            ..crate::debt::Rule::new(0, 1000)
        }],
        None,
    )
}
