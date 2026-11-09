use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "APLE",
        1418121,
        &[
            "ir.applehospitalityreit.com",
            "www.applehospitalityreit.com",
        ],
        vec![
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^(?:ffo|funds from operations) per (?:diluted )?share$",
                1,
            ),
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^(?:mffo|modified funds from operations) per (?:diluted )?share$",
                1,
            ),
            (Metric::Ffo, Unit::Total, r"^funds from operations$", 1000),
        ],
        None,
    ))?;
    company.debt = Some(debt);
    Ok(company)
}

fn debt(html: &scraper::Html, as_of: chrono::NaiveDate) -> anyhow::Result<crate::debt::Debt> {
    crate::debt::tables(html, as_of, &[crate::debt::Rule::new(0, 1000)], None)
}
