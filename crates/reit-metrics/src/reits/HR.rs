use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "HR",
        1360604,
        &["investors.healthcarerealty.com", "www.healthcarerealty.com"],
        vec![
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^ffo attributable to common stockholders per diluted share$",
                1,
            ),
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^(?:nareit )?ffo per common share\s*[-–—]\s*diluted$",
                1,
            ),
            (
                Metric::Ffo,
                Unit::Total,
                r"^(?:nareit )?ffo(?: attributable to common stockholders)?$",
                1000,
            ),
        ],
        None,
    ))?;
    company.tables = tables;
    Ok(company)
}

fn tables(html: &str) -> Vec<crate::table::Table> {
    let quarter = regex::Regex::new(r"(?i)^(FIRST|SECOND|THIRD|FOURTH) QUARTER ENDED$").unwrap();
    let short = regex::Regex::new(r"^(\d)Q (20\d{2})$").unwrap();
    crate::table::joined_parentheses(html)
        .into_iter()
        .filter_map(|mut table| {
            let reported = table
                .rows
                .iter()
                .take(6)
                .flatten()
                .find(|cell| cell.text == "AS REPORTED")
                .cloned();
            if reported.is_none()
                && table
                    .rows
                    .iter()
                    .take(6)
                    .flatten()
                    .any(|cell| cell.text.to_lowercase().contains("proforma"))
            {
                return None;
            }
            let month = table
                .rows
                .iter()
                .take(6)
                .flatten()
                .find_map(|cell| quarter.captures(&cell.text))
                .map(|matched| match matched[1].to_uppercase().as_str() {
                    "FIRST" => ("Three", "March 31"),
                    "SECOND" => ("Six", "June 30"),
                    "THIRD" => ("Nine", "September 30"),
                    _ => ("Twelve", "December 31"),
                });
            for cell in table.rows.iter_mut().flatten() {
                if let Some(reported) = &reported
                    && cell.column >= reported.column
                    && !reported.covers(cell.column)
                    && crate::number(&cell.text).is_some()
                {
                    cell.text.clear();
                }
                if let Some(matched) = short.captures(&cell.text) {
                    cell.text = format!("Q{} {}", &matched[1], &matched[2]);
                }
                if let Some((months, end)) = month {
                    if quarter.is_match(&cell.text) {
                        cell.text = format!("Three months ended {end}");
                    } else if cell.text == "YEAR TO DATE" {
                        cell.text = format!("{months} months ended {end}");
                    } else if cell.text == "FULL YEAR" {
                        cell.text = "Year ended December 31".into();
                    }
                }
            }
            Some(table)
        })
        .collect()
}
