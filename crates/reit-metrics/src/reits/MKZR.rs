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
    company.debt = Some(debt);
    company.ownership = Some(crate::ownership::Rules {
        prose: Some(ownership_prose),
        ..Default::default()
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

fn debt(_html: &scraper::Html, _as_of: chrono::NaiveDate) -> anyhow::Result<crate::debt::Debt> {
    anyhow::bail!("property-level borrowing schedules require consolidation")
}

fn ownership_prose(
    text: &str,
    source: &str,
    filed: Option<chrono::NaiveDate>,
    result: &mut std::collections::BTreeMap<chrono::NaiveDate, crate::ownership::Ownership>,
) -> anyhow::Result<()> {
    use crate::ownership::{Count, date};
    use regex::Regex;
    for matched in Regex::new(r"(?i)As of (?<date>[A-Za-z]+ \d{1,2}, 20\d{2}), we own all limited partnership units of the Operating Partnership except for (?<count>[\d,.]+) Class A Limited Partnership units")?.captures_iter(text) {
                if let Some(end) = date(&matched["date"]) {
                    result.entry(end).or_default().outside_op_units = Some(Count {
                        value: matched["count"].replace(',', "").parse()?, source: source.into(), evidence: matched[0].into(), filed,
                    });
                    let exchange = Regex::new(&format!(r"(?i)the {} Class A Limited Partnership units are convertible into the Company.s common stock on a (\d+):\s*(\d+) basis", regex::escape(&matched["count"])))?;
                    if let Some(exchange) = exchange.captures(text) {
                        let ratio = exchange[2].parse::<f64>()? / exchange[1].parse::<f64>()?;
                        if ratio.is_finite() && ratio > 0.0 {
                            let count = result[&end].outside_op_units.as_ref().unwrap();
                            let equivalents = Count { value: count.value / exchange[1].parse::<f64>()? * exchange[2].parse::<f64>()?, evidence: format!("{}; {}", count.evidence, &exchange[0]), ..count.clone() };
                            result.entry(end).or_default().outside_op_unit_share_equivalents = Some(equivalents);
                        }
                    }
                }
            }

    Ok(())
}
