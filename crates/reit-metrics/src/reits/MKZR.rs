use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "MKZR",
        1550913,
        &["www.mackenzierealty.com", "mackenzierealty.com"],
        vec![(
            Metric::Ffo,
            Unit::Total,
            r"^(?:ffo|funds from operations(?:\s*[-–—]\s*ffo)?)$",
            1,
        )],
        None,
    ))?;
    company.tables = tables;
    company.filings = &["8-K", "8-K/A", "10-Q", "10-Q/A", "10-K", "10-K/A"];
    company.additional_items = Some(|items| {
        items
            .split(',')
            .any(|item| matches!(item.trim(), "7.01" | "8.01"))
    });
    Ok(company)
}

fn tables(html: &str) -> Vec<crate::table::Table> {
    use crate::table::{Cell, Table};
    use chrono::{Datelike, Months, NaiveDate};

    let text = crate::table::normalize(
        &scraper::Html::parse_document(html)
            .root_element()
            .text()
            .collect::<Vec<_>>()
            .join(" "),
    );
    let period = regex::Regex::new(
        r"(?i)financial results for the (?:(?:first|second|third|fourth|fiscal) )?(quarter|year) ended ([A-Za-z]+ \d{1,2},? 20\d{2})",
    ).unwrap();
    let Some(period) = period.captures(&text) else {
        return Vec::new();
    };
    let Ok(end) = NaiveDate::parse_from_str(&period[2].replace(',', ""), "%B %d %Y") else {
        return Vec::new();
    };
    crate::table::joined_parentheses(html)
        .into_iter()
        .flat_map(|table| table.rows)
        .filter(|row| {
            row.iter()
                .find(|cell| !cell.text.is_empty())
                .is_some_and(|cell| cell.text.eq_ignore_ascii_case("ffo"))
        })
        .filter_map(|row| {
            let values = row
                .iter()
                .filter(|cell| crate::number(&cell.text).is_some())
                .collect::<Vec<_>>();
            if values.len() != 2 {
                return None;
            }
            let headers = values
                .into_iter()
                .enumerate()
                .map(|(index, cell)| {
                    let end = end.with_year(end.year() - index as i32).unwrap();
                    let start = end
                        .with_day(1)
                        .unwrap()
                        .checked_sub_months(Months::new(
                            if period[1].eq_ignore_ascii_case("year") {
                                11
                            } else {
                                2
                            },
                        ))
                        .unwrap();
                    Cell {
                        text: format!(
                            "For the period {} to {}",
                            start.format("%B %d, %Y"),
                            end.format("%B %d, %Y")
                        ),
                        column: cell.column,
                        width: cell.width,
                    }
                })
                .collect();
            Some(Table {
                rows: vec![headers, row],
                amount_scale: Some(1),
            })
        })
        .collect()
}
