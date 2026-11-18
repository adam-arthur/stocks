use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "GLPI",
        1575965,
        &[],
        vec![
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^ffo, per diluted common share(?: and op(?:/ltip)? units)?$",
                1,
            ),
            (
                Metric::Ffo,
                Unit::Total,
                r"^consolidated funds from operations$",
                0,
            ),
        ],
        None,
    ))?;
    company.tables = tables;
    company.rules.extend([
        crate::companies::Rule {
            metric: Metric::Affo,
            unit: Unit::PerShare,
            label: regex::Regex::new(
                r"(?i)^affo, per diluted common share(?: and op(?:/ltip)? units)?$",
            )?,
            scale: Some(1),
            section: None,
        },
        crate::companies::Rule {
            metric: Metric::Affo,
            unit: Unit::Total,
            label: regex::Regex::new(r"^consolidated adjusted funds from operations$")?,
            scale: Some(1000),
            section: None,
        },
    ]);
    company.affo_support = crate::AffoSupport::Reported;
    company.debt = Some(debt);
    Ok(company)
}

fn tables(html: &str) -> Vec<crate::table::Table> {
    let document = scraper::Html::parse_document(html);
    let selector = scraper::Selector::parse("table").unwrap();
    let consolidated =
        regex::Regex::new(r"(?i)\bconsolidated\s*\(in thousands[^)]*\)\s*\(unaudited\)\s*$")
            .unwrap();
    let mut text = String::new();
    let mut headings = Vec::new();
    for node in document.root_element().descendants() {
        if let Some(element) = scraper::ElementRef::wrap(node)
            && element.value().name() == "table"
            && element.select(&selector).next().is_none()
        {
            headings.push(consolidated.is_match(&crate::table::normalize(&text)));
        }
        if let Some(value) = node.value().as_text() {
            text.push_str(value);
            text.push(' ');
        }
    }
    let mut tables = crate::table::joined_parentheses(html);
    crate::table::align_year_pairs(&mut tables);
    tables
        .into_iter()
        .zip(headings)
        .filter_map(|(mut table, consolidated)| {
            if consolidated {
                for cell in table.rows.iter_mut().flatten() {
                    if cell
                        .text
                        .eq_ignore_ascii_case("Adjusted funds from operations")
                    {
                        cell.text = "Consolidated adjusted funds from operations".into();
                    } else if cell.text.eq_ignore_ascii_case("Funds from operations") {
                        cell.text = "Consolidated funds from operations".into();
                    }
                }
            }
            let actual = table
                .rows
                .iter()
                .take(6)
                .flatten()
                .filter(|cell| cell.text.ends_with("Actual"))
                .cloned()
                .collect::<Vec<_>>();
            if !actual.is_empty() {
                if table
                    .rows
                    .iter()
                    .take(6)
                    .flatten()
                    .any(|cell| cell.text.contains("Months Ending"))
                {
                    return None;
                }
                for cell in table.rows.iter_mut().flatten() {
                    if cell.text.to_lowercase().contains("guidance")
                        || (crate::number(&cell.text).is_some()
                            && !actual.iter().any(|header| header.covers(cell.column)))
                    {
                        cell.text.clear();
                    }
                }
            }
            Some(table)
        })
        .collect()
}

fn debt(html: &scraper::Html, as_of: chrono::NaiveDate) -> anyhow::Result<crate::debt::Debt> {
    crate::debt::tables(
        html,
        as_of,
        &[crate::debt::Rule {
            anchor: "",
            intro: "schedule of future minimum repayments of long-term debt",
            ..crate::debt::Rule::new(0, 1000)
        }],
        None,
    )
}
