use std::{path::Path, process::Command};

use anyhow::{Context, Result, ensure};
use chrono::NaiveDate;
use regex::Regex;

use crate::{Metric, Observation, Unit, number};

pub(crate) struct Rules {
    pub page: fn(&str) -> bool,
    pub metric: fn(&str, &str) -> Option<(Metric, Unit)>,
    pub total_scale: u64,
    pub current_only: fn(&str, &str, usize, usize) -> bool,
}

pub fn extract(rules: &Rules, path: &Path, source: &str) -> Result<Vec<Observation>> {
    let output = Command::new("pdftotext")
        .args(["-layout", "-enc", "UTF-8"])
        .arg(path)
        .arg("-")
        .output()
        .context("PDF extraction requires pdftotext (Poppler)")?;
    ensure!(
        output.status.success(),
        "pdftotext failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = String::from_utf8(output.stdout)?;
    let year = Regex::new(r"\b20\d{2}\b")?;
    let date = Regex::new(r"(?i)(mar(?:ch)?|jun(?:e)?|sep(?:tember)?|dec(?:ember)?)\.?\s+(31|30)")?;
    let gap = Regex::new(r"\s{2,}")?;
    let numeric = Regex::new(r"(\(?-?\d[\d,]*(?:\.\d+)?\)?)\s*(%?)")?;
    let mut result = Vec::new();
    for (page, text) in text.split('\u{c}').enumerate() {
        let compact_text = compact(text);
        if !(rules.page)(&compact_text) {
            continue;
        }
        let lines: Vec<_> = text.lines().collect();
        let Some(header_start) = lines
            .iter()
            .position(|line| compact(line).contains("threemonthsended"))
        else {
            continue;
        };
        let Some(year_row) = (header_start..lines.len().min(header_start + 5))
            .find(|index| matches!(year.find_iter(lines[*index]).count(), 2 | 4))
        else {
            continue;
        };
        let years: Vec<i32> = year
            .find_iter(lines[year_row])
            .map(|value| value.as_str().parse())
            .collect::<std::result::Result<_, _>>()?;
        if years[0] != years[1] + 1 || (years.len() == 4 && years[..2] != years[2..]) {
            continue;
        }
        let header = lines[header_start..=year_row].join(" ");
        let Some(date_match) = date.captures(&header) else {
            continue;
        };
        if date
            .captures_iter(&header)
            .any(|other| other[1] != date_match[1] || other[2] != date_match[2])
        {
            continue;
        }
        let month = match date_match[1][..3].to_lowercase().as_str() {
            "mar" => 3,
            "jun" => 6,
            "sep" => 9,
            "dec" => 12,
            _ => continue,
        };
        let day = date_match[2].parse::<u32>()?;
        let ytd = if compact(&header).contains("sixmonths") {
            6
        } else if compact(&header).contains("ninemonths") {
            9
        } else if compact(&header).contains("twelvemonths")
            || compact(&header).contains("yearended")
        {
            12
        } else {
            0
        };
        if years.len() == 4 && ytd == 0 {
            continue;
        }
        for (row, line) in lines.iter().enumerate().skip(year_row + 1) {
            let cells: Vec<_> = gap.split(line.trim()).collect();
            let label = compact(cells[0]);
            let Some((metric, unit)) = (rules.metric)(&label, &compact_text) else {
                continue;
            };
            let numbers: Option<Vec<_>> = numeric
                .captures_iter(&cells[1..].join(" "))
                .filter(|capture| capture[2].is_empty())
                .map(|capture| number(&capture[1]))
                .collect();
            let Some(numbers) = numbers else {
                continue;
            };
            let current_only =
                (rules.current_only)(line, lines[year_row], numbers.len(), years.len());
            if numbers.len() != years.len() && !current_only {
                continue;
            }
            for ((index, year), value) in years
                .iter()
                .enumerate()
                .filter(|(index, _)| !current_only || index % 2 == 0)
                .zip(numbers)
            {
                let months = if index < 2 { 3 } else { ytd };
                if months > month {
                    continue;
                }
                let Some(start) = NaiveDate::from_ymd_opt(*year, month + 1 - months, 1) else {
                    continue;
                };
                let Some(end) = NaiveDate::from_ymd_opt(*year, month, day) else {
                    continue;
                };
                result.push(Observation {
                    published_on: None,
                    metric,
                    start,
                    end,
                    label: cells[0].to_owned(),
                    value,
                    unit,
                    scale: if unit == Unit::Total {
                        rules.total_scale
                    } else {
                        1
                    },
                    source: source.to_owned(),
                    location: format!("PDF page {}, line {}", page + 1, row + 1),
                });
            }
        }
    }
    Ok(result)
}

fn compact(text: &str) -> String {
    text.chars()
        .filter(|character| !character.is_whitespace())
        .flat_map(char::to_lowercase)
        .collect()
}
