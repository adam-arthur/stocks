use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "SVC",
        945394,
        &[
            "svcreit.com",
            "www.svcreit.com",
            "hptreit.com",
            "www.hptreit.com",
        ],
        vec![
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^ffo available for common shareholders (?:basic and )?diluted$",
                1,
            ),
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^ffo(?: available for common shareholders)?$",
                1,
            ),
        ],
        None,
    ))?;
    company.rules[1].section = Some(regex::Regex::new(
        r"(?i)^basic and diluted per common share amounts:",
    )?);
    company.tables = tables;
    company.extra_observations = Some(super::dhc::extract);
    company.debt = Some(debt);
    company.affo_support = crate::AffoSupport::Reported;
    Ok(company)
}

pub(crate) fn tables(html: &str) -> Vec<crate::table::Table> {
    let mut tables = crate::table::joined_parentheses(html);
    for cell in tables
        .iter_mut()
        .flat_map(|table| table.rows.iter_mut().flatten())
    {
        cell.text = cell
            .text
            .replace("(basic)", "basic")
            .replace("(diluted)", "diluted")
            .replace("(basic and diluted)", "basic and diluted");
    }
    tables
}

fn debt(html: &scraper::Html, as_of: chrono::NaiveDate) -> anyhow::Result<crate::debt::Debt> {
    crate::debt::tables(html, as_of, &[crate::debt::Rule::new(0, 1000)], None)
}
