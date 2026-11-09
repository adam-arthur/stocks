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
