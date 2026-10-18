use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "CCI",
        1051470,
        &["www.crowncastle.com", "investor.crowncastle.com"],
        vec![
            (Metric::Ffo, Unit::PerShare, r"^ffo per share$", 1),
            (Metric::Ffo, Unit::Total, r"^ffo$", 0),
        ],
        None,
    ))?;
    company.tables = tables;
    Ok(company)
}

fn tables(html: &str) -> Vec<crate::table::Table> {
    let document = scraper::Html::parse_document(html);
    let selector = scraper::Selector::parse("table").unwrap();
    let mut text = String::new();
    let mut offsets = Vec::new();
    for node in document.root_element().descendants() {
        if let Some(element) = scraper::ElementRef::wrap(node)
            && element.value().name() == "table"
            && element.select(&selector).next().is_none()
        {
            offsets.push(text.len());
        }
        if let Some(value) = node.value().as_text() {
            text.push_str(&crate::table::normalize(value));
            text.push(' ');
        }
    }
    let mut tables = crate::table::joined_parentheses(html);
    for caption in regex::Regex::new(r"(?i)reconciliation of historical ffo and affo per share")
        .unwrap()
        .find_iter(&text)
    {
        if let Some(table) =
            tables.get_mut(offsets.partition_point(|offset| *offset < caption.end()))
        {
            for cell in table.rows.iter_mut().filter_map(|row| row.first_mut()) {
                if cell.text.starts_with("FFO (") && !cell.text.contains("from above") {
                    cell.text = format!("FFO per share{}", &cell.text[3..]);
                }
            }
        }
    }
    tables
}
