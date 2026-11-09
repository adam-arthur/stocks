use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "IIPR",
        1677576,
        &[
            "investors.innovativeindustrialproperties.com",
            "www.innovativeindustrialproperties.com",
        ],
        vec![
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^ffo per (?:common )?share\s*[-–—]\s*(?:basic and )?diluted$",
                1,
            ),
            (
                Metric::Ffo,
                Unit::Total,
                r"^ffo(?: attributable to common stockholders(?: (?:basic and )?diluted)?)?$",
                1000,
            ),
        ],
        None,
    ))?;
    company.tables = tables;
    company.debt = Some(debt);
    Ok(company)
}

fn tables(html: &str) -> Vec<crate::table::Table> {
    let mut tables = crate::table::joined_parentheses(html);
    let text = crate::table::normalize(
        &scraper::Html::parse_document(html)
            .root_element()
            .text()
            .collect::<Vec<_>>()
            .join(" "),
    );
    let caption = regex::Regex::new(r"(?i)FFO available to common shares and a reconciliation to net loss for the three months ended ((?:March|June|September|December)\s+\d{1,2},?\s+20\d{2}) were as follows").unwrap();
    if let Some(caption) = caption.captures(&text) {
        let year = regex::Regex::new(r"\b20\d{2}\b").unwrap();
        for table in &mut tables {
            if table.rows.iter().flatten().any(|cell| cell.text == "FFO")
                && !table
                    .rows
                    .iter()
                    .take(4)
                    .flatten()
                    .any(|cell| year.is_match(&cell.text))
            {
                table.rows.insert(
                    0,
                    vec![crate::table::Cell {
                        text: format!("Three months ended {}", &caption[1]),
                        column: 0,
                        width: usize::MAX,
                    }],
                );
            }
        }
    }
    for row in tables.iter_mut().flat_map(|table| &mut table.rows) {
        if let Some(label) = row.iter_mut().find(|cell| !cell.text.is_empty())
            && label
                .text
                .starts_with("FFO attributable to common stockholders")
        {
            label.text = label.text.replace(['(', ')'], "");
        }
    }
    tables
}

fn debt(html: &scraper::Html, as_of: chrono::NaiveDate) -> anyhow::Result<crate::debt::Debt> {
    crate::debt::tables(html, as_of, &[crate::debt::Rule::new(0, 1000)], None)
}
