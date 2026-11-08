use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "BHM",
        1903382,
        &["bluerock.com", "bhm.bluerock.com"],
        vec![
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^ffo attributable to common stockholders and unit holders\s*[-–—]\s*diluted$",
                1,
            ),
            (
                Metric::Ffo,
                Unit::Total,
                r"^ffo attributable to common stockholders and unit holders$",
                1000,
            ),
        ],
        None,
    ))?;
    company.filings = &["10-Q", "10-Q/A", "10-K", "10-K/A"];
    company.tables = tables;
    company.derive = Some(|documents| crate::derive::fourth_quarters("BHM", documents));
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
    let mut tables = crate::table::tables(html);
    for caption in regex::Regex::new(
        r"(?i)calculation\s+of\s+FFO\s+and\s+CFFO\s+for\s+the\s+years\s+ended\s+December\s+31,\s+(20\d{2})\s+and\s+(20\d{2})",
    ).unwrap().captures_iter(&text) {
        let index = offsets.partition_point(|offset| *offset < caption.get(0).unwrap().end());
        if let Some(table) = tables.get_mut(index)
            && table.rows.iter().flatten().any(|cell| cell.text.to_lowercase().starts_with("ffo attributable to common stockholders and unit holders"))
            && [&caption[1], &caption[2]].iter().all(|year| table.rows.iter().take(6).flatten().any(|cell| cell.text == *year))
        {
            table.rows.insert(0, vec![crate::table::Cell {
                text: "Years ended December 31,".into(),
                column: 0,
                width: usize::MAX,
            }]);
        }
    }
    tables
}
