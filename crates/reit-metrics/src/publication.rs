use std::collections::BTreeSet;

use anyhow::{Context, Result, ensure};
use chrono::NaiveDate;
use regex::Regex;
use scraper::Html;

pub fn parse(value: &str) -> Result<NaiveDate> {
    let value = value.replace(['.', ','], " ");
    let value = value.split_whitespace().collect::<Vec<_>>().join(" ");
    let value = Regex::new(r"(?i)^Sept ")?.replace(&value, "Sep ");
    let value = Regex::new(r"^([A-Za-z]+)\s+(\d(?:\s*\d)?)\s+(2\s*0\s*\d\s*\d)$")?
        .captures(&value)
        .map(|found| {
            format!(
                "{} {} {}",
                &found[1],
                found[2].replace(' ', ""),
                found[3].replace(' ', "")
            )
        })
        .unwrap_or_else(|| value.to_string());
    ["%B %d %Y", "%b %d %Y", "%Y-%m-%d"]
        .into_iter()
        .find_map(|format| NaiveDate::parse_from_str(&value, format).ok())
        .with_context(|| format!("invalid publication date: {value}"))
}

pub fn extract(html: &str, filed_on: NaiveDate) -> Result<NaiveDate> {
    let html = Html::parse_document(html);
    let text = html
        .root_element()
        .descendants()
        .filter(|node| {
            !node.ancestors().any(|ancestor| {
                ancestor.value().as_element().is_some_and(|element| {
                    matches!(element.name(), "script" | "style" | "head" | "ix:header")
                })
            })
        })
        .fold(String::new(), |mut text, node| {
            if let Some(value) = node.value().as_text() {
                text.push(' ');
                text.push_str(value);
            } else if node.value().as_element().is_some_and(|element| {
                matches!(
                    element.name(),
                    "p" | "div" | "tr" | "td" | "br" | "h1" | "h2" | "h3"
                )
            }) {
                text.push('\n');
            }
            text
        });
    let text = text.replace('\u{200b}', "");
    let text = text
        .lines()
        .map(|line| line.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n");
    let heading = Regex::new(r"(?im)^\s*item\s+([1-9])\s*\.\s*(\d{2})\b|^\s*signatures?\b")?;
    let headings = heading.captures_iter(&text).collect::<Vec<_>>();
    let date = r"(?P<date>(?:Jan(?:uary)?|Feb(?:ruary)?|Mar(?:ch)?|Apr(?:il)?|May|Jun(?:e)?|Jul(?:y)?|Aug(?:ust)?|Sep(?:t(?:ember)?)?|Oct(?:ober)?|Nov(?:ember)?|Dec(?:ember)?)\.?\s+\d(?:\s*\d)?\s*,?\s+2\s*0\s*\d\s*\d|20\d{2}-\d{2}-\d{2})";
    let patterns = [
        format!(r"(?i)\bon\s+{date}\s*,?\s+[^;!?]{{0,250}}?\b(?:issued|announced|reported|released|published|furnished\s+the\s+following\s+documents|made\s+[^.;!?]{{0,70}}?\b(?:earnings|press|news)\s+release)\b"),
        format!(r"(?i)\b(?:press|earnings|news)\s+release(?:\s+and\s+supplemental\s+information)?(?:\s+(?:of|issued\s+by)\s+[^;!?]{{0,150}}?)?\s*,?\s+(?:dated|issued|published|on)\s+{date}"),
    ]
    .into_iter()
    .map(|pattern| Regex::new(&pattern))
    .collect::<std::result::Result<Vec<_>, _>>()?;
    let combined = Regex::new(
        r"(?i)^[.\s–—-]*results\s+of\s+operations\s+and\s+financial\s+condition(?:\s*\([^)]*\))?[.\s]*(?:and\s*)?$",
    )?;
    let mut dates = BTreeSet::new();
    for (index, found) in headings.iter().enumerate().filter(|(_, found)| {
        found.get(1).is_some_and(|value| value.as_str() == "2")
            && found.get(2).is_some_and(|value| value.as_str() == "02")
    }) {
        let mut next = index + 1;
        while next < headings.len()
            && headings[next]
                .get(1)
                .is_some_and(|value| value.as_str() == "7")
            && headings[next]
                .get(2)
                .is_some_and(|value| value.as_str() == "01")
            && combined.is_match(
                &text[headings[next - 1].get(0).unwrap().end()
                    ..headings[next].get(0).unwrap().start()],
            )
        {
            next += 1;
        }
        let section = &text[found.get(0).unwrap().end()
            ..headings
                .get(next)
                .map_or(text.len(), |next| next.get(0).unwrap().start())];
        for pattern in &patterns {
            for found in pattern.captures_iter(section) {
                dates.insert(parse(&found["date"])?);
            }
        }
    }
    ensure!(!dates.is_empty(), "missing announcement date in Item 2.02");
    ensure!(
        dates.len() == 1,
        "conflicting announcement dates in Item 2.02: {dates:?}"
    );
    let date = dates.into_iter().next().unwrap();
    ensure!(
        date <= filed_on,
        "announcement date {date} is after filing date {filed_on}"
    );
    Ok(date)
}
