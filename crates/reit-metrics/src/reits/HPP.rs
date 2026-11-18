use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "HPP",
        1482512,
        &[
            "investors.hudsonpacificproperties.com",
            "www.hudsonpacificproperties.com",
        ],
        vec![
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^ffo per common stock/unit\s*[-–—]\s*diluted$",
                1,
            ),
            (
                Metric::Ffo,
                Unit::Total,
                r"^ffo to common (?:stockholders and unitholders|stock/unit holders)$",
                1000,
            ),
        ],
        None,
    ))?;
    company.tables = tables;
    company.extra_observations = Some(highlights);
    company.rules.extend([
        crate::companies::Rule {
            metric: Metric::Affo,
            unit: Unit::PerShare,
            label: regex::Regex::new(r"(?i)^affo per common stock/unit\s*[-–—]\s*diluted$")?,
            scale: Some(1),
            section: None,
        },
        crate::companies::Rule {
            metric: Metric::Affo,
            unit: Unit::Total,
            label: regex::Regex::new(r"(?i)^affo$")?,
            scale: Some(1000),
            section: None,
        },
    ]);
    company.affo_support = crate::AffoSupport::Reported;
    company.debt = Some(debt);
    Ok(company)
}

fn highlights(html: &str, source: &str) -> Result<Vec<crate::Observation>> {
    let text = crate::table::normalize(
        &scraper::Html::parse_document(html)
            .root_element()
            .text()
            .collect::<Vec<_>>()
            .join(" "),
    );
    let title = regex::Regex::new(
        r"(?i)Hudson Pacific Properties Reports (First|Second|Third|Fourth) Quarter (20[0-9]{2}) Financial Results",
    )?;
    let Some(title) = title.captures(&text) else {
        return Ok(Vec::new());
    };
    let metric = regex::Regex::new(
        r"(?i)FFO, including specified items, of \$[0-9.]+ million, or \$([0-9.]+) per diluted share",
    )?;
    let Some(value) = metric.captures(&text) else {
        return Ok(Vec::new());
    };
    let start = chrono::NaiveDate::from_ymd_opt(
        title[2].parse()?,
        match title[1].to_ascii_lowercase().as_str() {
            "first" => 1,
            "second" => 4,
            "third" => 7,
            _ => 10,
        },
        1,
    )
    .unwrap();
    Ok(vec![crate::Observation {
        metric: Metric::Ffo,
        unit: Unit::PerShare,
        scale: 1,
        label: "FFO including specified items per diluted share".into(),
        value: value[1].parse()?,
        source: source.into(),
        published_on: None,
        start,
        end: start
            .checked_add_months(chrono::Months::new(3))
            .unwrap()
            .pred_opt()
            .unwrap(),
        location: "earnings release highlights".into(),
    }])
}

fn debt(html: &scraper::Html, as_of: chrono::NaiveDate) -> anyhow::Result<crate::debt::Debt> {
    crate::debt::tables(
        html,
        as_of,
        &[crate::debt::Rule {
            sum_columns: 2,
            ..crate::debt::Rule::new(0, 1000)
        }],
        None,
    )
}

fn tables(html: &str) -> Vec<crate::table::Table> {
    let mut tables = crate::table::joined_parentheses(html);
    let document = scraper::Html::parse_document(html);
    let dates =
        regex::Regex::new(r"(?:March 31|June 30|September 30|December 31), 20\d{2}").unwrap();
    let sections = regex::Regex::new(r"(?s)(Quarter To Date|Year To Date) (.*?)AFFO\s+((?:\$\s*\(?-?[\d,]+\)?\s*){5})Dividends paid").unwrap();
    let numbers = regex::Regex::new(r"\$\s*(\(?-?[\d,]+\)?)").unwrap();
    for node in document.select(&scraper::Selector::parse("font").unwrap()) {
        let text = crate::table::normalize(&node.text().collect::<String>());
        if !text.contains("Recurring capital expenditures")
            || !text.contains("Unaudited, in thousands")
        {
            continue;
        }
        for section in sections.captures_iter(&text) {
            let ends: Vec<_> = dates.find_iter(&section[2]).take(5).collect();
            let values: Vec<_> = numbers.captures_iter(&section[3]).collect();
            if ends.len() != 5 || values.len() != 5 {
                continue;
            }
            for (end, value) in ends.iter().zip(values) {
                let duration = if &section[1] == "Quarter To Date" {
                    "Three"
                } else if end.as_str().starts_with("March") {
                    "Three"
                } else if end.as_str().starts_with("June") {
                    "Six"
                } else if end.as_str().starts_with("September") {
                    "Nine"
                } else {
                    "Twelve"
                };
                tables.push(crate::table::Table {
                    amount_scale: Some(1000),
                    rows: vec![
                        vec![crate::table::Cell {
                            text: format!("{duration} months ended {}", end.as_str()),
                            column: 1,
                            width: 1,
                        }],
                        vec![
                            crate::table::Cell {
                                text: "AFFO".into(),
                                column: 0,
                                width: 1,
                            },
                            crate::table::Cell {
                                text: value[1].into(),
                                column: 1,
                                width: 1,
                            },
                        ],
                    ],
                });
            }
        }
    }
    tables
}
