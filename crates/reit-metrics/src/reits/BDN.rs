use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "BDN",
        790816,
        &["investor.brandywinerealty.com", "www.brandywinerealty.com"],
        vec![
            (
                Metric::Affo,
                Unit::Total,
                r"^cash available for distribution$",
                1000,
            ),
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^ffo per share\s*[-–—]\s*(?:basic/)?fully diluted$",
                1,
            ),
            (
                Metric::Ffo,
                Unit::Total,
                r"^funds from operations available to common share and unit holders$",
                1000,
            ),
        ],
        None,
    ))?;
    company.debt = Some(debt);
    company.affo_support = crate::AffoSupport::Reported;
    company.filings = &["8-K", "8-K/A", "DEF 14A"];
    company.tables = tables;
    company.sections.push(affo_per_share);
    Ok(company)
}

fn debt(html: &scraper::Html, as_of: chrono::NaiveDate) -> anyhow::Result<crate::debt::Debt> {
    crate::debt::tables(
        html,
        as_of,
        &[crate::debt::Rule {
            intro: "aggregate scheduled principal payments on",
            ..crate::debt::Rule::new(0, 1000)
        }],
        None,
    )
}

fn affo_per_share(tables: &[crate::table::Table], source: &str) -> Result<Vec<crate::Observation>> {
    Ok(crate::affo_per_share(
        company()?,
        tables,
        source,
        crate::companies::Rule {
            metric: Metric::Affo,
            unit: Unit::Total,
            label: regex::Regex::new(
                r"^weighted-average shares/units outstanding - fully diluted$",
            )?,
            scale: Some(1),
            section: None,
        },
    )?
    .into_iter()
    .filter(|value| {
        value.start
            == chrono::NaiveDate::from_ymd_opt(chrono::Datelike::year(&value.end), 1, 1).unwrap()
            && chrono::Datelike::month(&value.end) == 12
    })
    .collect())
}

fn tables(html: &str) -> Vec<crate::table::Table> {
    let document = scraper::Html::parse_document(html);
    let mut text = String::new();
    let mut offsets = std::collections::HashMap::new();
    for node in document.root_element().descendants() {
        if node
            .value()
            .as_element()
            .is_some_and(|element| element.name() == "table")
        {
            offsets.insert(node.id(), text.len());
        }
        if let Some(value) = node.value().as_text() {
            text.push_str(&crate::table::normalize(value));
            text.push(' ');
        }
    }
    let selector = scraper::Selector::parse("table").unwrap();
    let annual = regex::Regex::new(r"(?i)twelve months ended\s+December 31,?\s+(20\d{2})").unwrap();
    let mut tables = crate::table::joined_parentheses(html);
    for (table, element) in tables.iter_mut().zip(
        document
            .select(&selector)
            .filter(|table| table.select(&selector).next().is_none()),
    ) {
        if !table.rows.iter().flatten().any(|cell| {
            let label = cell.text.to_lowercase();
            label.starts_with("cash available for distribution")
                || label.starts_with("weighted-average shares/units outstanding")
        }) {
            continue;
        }
        let offset = offsets[&element.id()];
        let preceding: String = text[..offset]
            .chars()
            .rev()
            .take(4000)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        let heading = format!(
            "{} {}",
            preceding,
            table
                .rows
                .iter()
                .take(10)
                .flatten()
                .map(|cell| cell.text.as_str())
                .collect::<Vec<_>>()
                .join(" ")
        );
        if let Some(year) = annual.captures_iter(&heading).last() {
            table.rows.insert(
                0,
                vec![crate::table::Cell {
                    text: format!("Twelve months ended December 31, {}", &year[1]),
                    column: 0,
                    width: 500,
                }],
            );
        }
    }
    tables
}
