use anyhow::Result;
use regex::Regex;
use scraper::{Html, Selector};

pub(crate) fn pages(html: &str) -> Result<Vec<String>> {
    let size = Regex::new(r"(?i)font-size:\s*1(?:\.0)?pt")?;
    Ok(Html::parse_document(html)
        .select(&Selector::parse("font").unwrap())
        .filter(|element| {
            element.value().attr("size") == Some("1")
                || size.is_match(element.value().attr("style").unwrap_or_default())
        })
        .map(|element| crate::table::normalize(&element.text().collect::<Vec<_>>().join(" ")))
        .collect())
}

pub(crate) fn values(text: &str, label: &str) -> Result<Vec<Vec<f64>>> {
    let rows = Regex::new(&format!(
        r"(?i)\b{label}(?:\s*\(\d+\))?\s+((?:(?:\$?\s*\(?-?\d[\d,]*(?:\.\d+)?\)?\s*%?|n/a)\s+)+)"
    ))?;
    let number = Regex::new(r"\$?\s*(\(?-?\d[\d,]*(?:\.\d+)?\)?)\s*(%)?")?;
    Ok(rows
        .captures_iter(text)
        .map(|row| {
            number
                .captures_iter(&row[1])
                .filter(|value| value.get(2).is_none())
                .filter_map(|value| crate::number(&value[1]))
                .collect()
        })
        .collect())
}
