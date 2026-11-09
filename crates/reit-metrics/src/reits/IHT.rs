use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "IHT",
        82473,
        &["www.innsuitestrust.com", "innsuitestrust.com"],
        vec![],
        None,
    ))?;
    company.extra_observations = Some(extract);
    company.derive = Some(quarters);
    company.filings = &["10-Q", "10-Q/A", "10-K", "10-K/A"];
    company.debt = Some(debt);
    company.ownership = Some(crate::ownership::Rules::default());
    Ok(company)
}

fn extract(html: &str, source: &str) -> Result<Vec<crate::Observation>> {
    use chrono::{Datelike, Months, NaiveDate};
    let date = regex::Regex::new(r"(?i)(January|April|July|October)\s+(30|31)")?;
    let year = regex::Regex::new(r"^20\d{2}$")?;
    let label = regex::Regex::new(
        r"(?i)^FFO(?: attributable to controlling(?: and non-controlling)? interests?)?$",
    )?;
    let text = crate::table::normalize(
        &scraper::Html::parse_document(html)
            .root_element()
            .text()
            .collect::<Vec<_>>()
            .join(" "),
    );
    let reported = regex::Regex::new(r"(?i)for the quarterly period ended\s+((?:January|April|July|October)\s+\d{1,2},?\s+20\d{2})")?
        .captures(&text).and_then(|value| NaiveDate::parse_from_str(&value[1].replace(',', ""), "%B %d %Y").ok());
    let mut result = Vec::new();
    for (table_number, table) in crate::table::joined_parentheses(html).iter().enumerate() {
        for (row_number, row) in table.rows.iter().enumerate() {
            let Some(first) = row.iter().find(|cell| !cell.text.is_empty()) else {
                continue;
            };
            if !label.is_match(&first.text) {
                continue;
            }
            for cell in row.iter().filter(|cell| cell.column > first.column) {
                let Some(value) = crate::number(&cell.text) else {
                    continue;
                };
                let Some(year) = table.rows[..row_number]
                    .iter()
                    .flatten()
                    .find(|header| header.covers(cell.column) && year.is_match(&header.text))
                    .and_then(|header| header.text.parse().ok())
                else {
                    continue;
                };
                let Some((months, month, day)) = table.rows[..row_number]
                    .iter()
                    .flatten()
                    .filter(|header| header.covers(cell.column))
                    .find_map(|header| {
                        let date = date.captures(&header.text)?;
                        Some((
                            crate::duration(&header.text)?,
                            match date[1].to_lowercase().as_str() {
                                "january" => 1,
                                "april" => 4,
                                "july" => 7,
                                _ => 10,
                            },
                            date[2].parse::<u32>().ok()?,
                        ))
                    })
                else {
                    continue;
                };
                let day = reported
                    .filter(|reported| {
                        reported.month() == month
                            && reported.succ_opt().is_some_and(|next| next.day() == 1)
                    })
                    .map_or(day, |reported| reported.day());
                let Some(end) = NaiveDate::from_ymd_opt(year, month, day) else {
                    continue;
                };
                let Some(start) = end
                    .succ_opt()
                    .and_then(|date| date.checked_sub_months(Months::new(months)))
                else {
                    continue;
                };
                result.push(crate::Observation {
                    metric: Metric::Ffo,
                    start,
                    end,
                    label: first.text.clone(),
                    value,
                    unit: Unit::Total,
                    scale: 1,
                    source: source.to_owned(),
                    location: format!("table {table_number}, row {row_number}"),
                });
            }
        }
    }
    Ok(result)
}

fn quarters(documents: &[crate::Extraction]) -> Vec<crate::Extraction> {
    use chrono::{Datelike, Months};
    let mut periods = std::collections::BTreeMap::<_, Vec<_>>::new();
    for document in documents {
        for value in document
            .observations
            .iter()
            .filter(|value| value.unit == Unit::Total)
        {
            periods
                .entry((value.start, value.end))
                .or_default()
                .push((document, value));
        }
    }
    periods
        .iter()
        .filter(|((start, end), _)| {
            start.month() == 2
                && start.day() == 1
                && [6, 9, 12]
                    .iter()
                    .any(|months| start.checked_add_months(Months::new(*months)) == end.succ_opt())
                && !periods.contains_key(&(
                    end.succ_opt()
                        .unwrap()
                        .checked_sub_months(Months::new(3))
                        .unwrap(),
                    *end,
                ))
        })
        .map(|((start, end), cumulative)| {
            let (document, current) = cumulative
                .iter()
                .min_by_key(|(document, value)| {
                    (document.report_period != Some(value.end), document.filed_on)
                })
                .unwrap();
            let mut result = crate::Extraction {
                observations: Vec::new(),
                report_period: Some(*end),
                ..(*document).clone()
            };
            let Some(preceding) = periods.get(&(
                *start,
                end.succ_opt()
                    .unwrap()
                    .checked_sub_months(Months::new(3))
                    .unwrap()
                    .pred_opt()
                    .unwrap(),
            )) else {
                return result;
            };
            if [cumulative, preceding].iter().any(|values| {
                values.iter().any(|(_, value)| {
                    value.value * value.scale as f64 != values[0].1.value * values[0].1.scale as f64
                })
            }) {
                return result;
            }
            let (quarterly, previous) = preceding
                .iter()
                .min_by_key(|(document, value)| {
                    (document.report_period != Some(value.end), document.filed_on)
                })
                .unwrap();
            result.filed_on = document
                .filed_on
                .zip(quarterly.filed_on)
                .map(|(current, quarterly)| current.max(quarterly));
            result.observations.push(crate::Observation {
                start: end
                    .succ_opt()
                    .unwrap()
                    .checked_sub_months(Months::new(3))
                    .unwrap(),
                label: format!("Calculated quarterly {}", current.label),
                value: current.value * current.scale as f64
                    - previous.value * previous.scale as f64,
                scale: 1,
                location: format!(
                    "calculated: cumulative {} ({}) minus preceding cumulative {} ({})",
                    current.source, current.location, previous.source, previous.location
                ),
                ..(*current).clone()
            });
            result
        })
        .collect()
}

fn debt(html: &scraper::Html, as_of: chrono::NaiveDate) -> anyhow::Result<crate::debt::Debt> {
    crate::debt::tables(
        html,
        as_of,
        &[crate::debt::Rule {
            sum_columns: 4,
            ..crate::debt::Rule::new(0, 1)
        }],
        None,
    )
}
