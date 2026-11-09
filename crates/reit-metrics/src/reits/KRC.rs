use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "KRC",
        1025996,
        &["www.kilroyrealty.com", "investors.kilroyrealty.com"],
        vec![
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^ffo per common share/unit\s*[-–—]\s*diluted$",
                1,
            ),
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^funds from operations per common share(?:/unit)?\s*[-–—]\s*diluted$",
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
