use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "CPT",
        906345,
        &["www.camdenliving.com", "investors.camdenliving.com"],
        vec![
            (
                Metric::Affo,
                Unit::PerShare,
                r#"^core (?:adjusted funds from operations - diluted|affo per common share - diluted)$"#,
                1,
            ),
            (
                Metric::Affo,
                Unit::Total,
                r#"^core adjusted funds from operations$"#,
                1000,
            ),
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^ffo per common share - diluted$",
                1,
            ),
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^funds from operations - diluted$",
                1,
            ),
        ],
        None,
    ))?;
    company.tables = crate::table::joined_parentheses;
    company.affo_support = crate::AffoSupport::Reported;
    company.debt = Some(debt);
    Ok(company)
}

fn debt(html: &scraper::Html, as_of: chrono::NaiveDate) -> anyhow::Result<crate::debt::Debt> {
    crate::debt::tables(html, as_of, &[crate::debt::Rule::new(0, 1000000)], None)
}
