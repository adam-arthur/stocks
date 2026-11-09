use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "HST",
        1070750,
        &["www.hosthotels.com", "ir.hosthotels.com"],
        vec![
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^nareit ffo(?: and adjusted ffo)? per diluted share$",
                1,
            ),
            (Metric::Ffo, Unit::Total, r"^diluted nareit ffo$", 1000000),
            (Metric::Ffo, Unit::Total, r"^nareit ffo$", 1000000),
        ],
        None,
    ))?;
    company.tables = tables;
    company.official_url = |url| {
        url.host_str() == Some("www.sec.gov")
            && url.path().starts_with("/Archives/edgar/data/1061937/")
    };
    company.debt = Some(debt);
    Ok(company)
}

fn tables(html: &str) -> Vec<crate::table::Table> {
    let footnote = regex::Regex::new(r"⁽[⁰¹²³⁴⁵⁶⁷⁸⁹]+⁾").unwrap();
    let ytd = regex::Regex::new(r"(?i)year-to-date ended (March|June|September|December)").unwrap();
    crate::table::joined_parentheses(html)
        .into_iter()
        .filter_map(|mut table| {
            if table
                .rows
                .iter()
                .take(6)
                .flatten()
                .any(|cell| cell.text.to_lowercase().contains("low-end of range"))
            {
                return None;
            }
            for cell in table.rows.iter_mut().flatten() {
                cell.text = footnote.replace_all(&cell.text, "").into_owned();
                if let Some(matched) = ytd.captures(&cell.text) {
                    let months = match matched[1].to_lowercase().as_str() {
                        "march" => "Three",
                        "june" => "Six",
                        "september" => "Nine",
                        _ => "Twelve",
                    };
                    cell.text = ytd
                        .replace(&cell.text, format!("{months} months ended {}", &matched[1]))
                        .into_owned();
                }
            }
            Some(table)
        })
        .collect()
}

fn debt(html: &scraper::Html, as_of: chrono::NaiveDate) -> anyhow::Result<crate::debt::Debt> {
    crate::debt::tables(html, as_of, &[crate::debt::Rule::new(0, 1000000)], None)
}
