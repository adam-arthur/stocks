use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "O",
        726728,
        &["www.realtyincome.com"][..],
        vec![
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^ffo per (?:diluted )?share$",
                1,
            ),
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^ffo per common share:? diluted$",
                1,
            ),
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^ffo per common share, basic and diluted$",
                1,
            ),
            (
                Metric::Ffo,
                Unit::Total,
                r"^funds from operations available to common stockholders \(ffo\)$",
                0,
            ),
        ],
        None,
    ))?;
    company.debt = Some(debt);
    Ok(company)
}

fn debt(html: &scraper::Html, as_of: chrono::NaiveDate) -> anyhow::Result<crate::debt::Debt> {
    crate::debt::tables(
        html,
        as_of,
        &[crate::debt::Rule {
            anchor: "",
            heading: "Total Consolidated Debt Principal",
            ..crate::debt::Rule::new(7, 1_000_000)
        }],
        None,
    )
}
