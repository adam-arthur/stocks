use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "REG",
        910606,
        &[
            "www.regencycenters.com",
            "regencycenters.com",
            "investors.regencycenters.com",
        ],
        vec![
            (
                Metric::Affo,
                Unit::Total,
                r"^adjusted (?:core )?funds from operations$",
                1000,
            ),
            (Metric::Ffo, Unit::PerShare, r"^nareit ffo per share$", 1),
            (
                Metric::Ffo,
                Unit::Total,
                r"^nareit (?:funds from operations(?: \(nareit ffo\))?|ffo)$",
                1000,
            ),
        ],
        None,
    ))?;
    company.tables = tables;
    company.extra_observations = Some(extract);
    company.debt = Some(debt);
    company.affo_comparatives = true;
    company.affo_support = crate::AffoSupport::Reported;
    company.sections.push(affo_per_share);
    Ok(company)
}

fn affo_per_share(tables: &[crate::table::Table], source: &str) -> Result<Vec<crate::Observation>> {
    crate::affo_per_share(
        company()?,
        tables,
        source,
        crate::companies::Rule {
            metric: Metric::Affo,
            unit: Unit::Total,
            label: regex::Regex::new(
                r"^weighted average shares (?:for diluted nareit ffo and core ffo per share|\(diluted\)\s*[-–—]\s*nareit ffo and core ffo)$",
            )?,
            scale: Some(1000),
            section: None,
        },
    )
}

fn tables(html: &str) -> Vec<crate::table::Table> {
    let mut tables = crate::table::joined_parentheses(html);
    let pattern =
        regex::Regex::new(r"(?i)for the periods ended ((March|June|September|December) (?:30|31))")
            .unwrap();
    let date = |text: &str| pattern.captures(text).map(|matched| matched[1].to_owned());
    let fallback = tables
        .iter()
        .flat_map(|table| &table.rows)
        .flatten()
        .find_map(|cell| date(&cell.text));
    let years = regex::Regex::new(r"(?i)for the periods ended (?:march|june|september|december) (?:30|31), (20\d{2}) and (20\d{2})").unwrap();
    for table in &mut tables {
        if let Some(per_share) = table.rows.iter().position(|row| {
            row.iter()
                .any(|c| c.text.to_lowercase().contains("ffo (per diluted share)"))
        }) {
            if table.rows[..per_share].iter().flatten().any(|c| {
                c.text
                    .to_lowercase()
                    .starts_with("reconciliation of net income")
            }) {
                table.rows.truncate(per_share);
            } else {
                table.rows.clear();
            }
        }
        if let Some(expected) = table
            .rows
            .iter()
            .take(2)
            .flatten()
            .find_map(|cell| years.captures(&cell.text))
            && let Some(first) = table.rows.iter().take(4).flatten().find(|c| {
                c.text.len() == 4 && c.text.starts_with("20") && c.text.parse::<u32>().is_ok()
            })
            && first.text != expected[1]
        {
            table.rows.clear();
            continue;
        }
        if let Some(date) = table
            .rows
            .iter()
            .take(8)
            .flatten()
            .find_map(|cell| date(&cell.text))
            .or_else(|| fallback.clone())
        {
            for cell in table.rows.iter_mut().take(8).flatten() {
                if cell.text.eq_ignore_ascii_case("year to date") {
                    cell.text = format!(
                        "{} {date}",
                        match date
                            .split_whitespace()
                            .next()
                            .unwrap()
                            .to_lowercase()
                            .as_str()
                        {
                            "march" => "Three Months Ended",
                            "june" => "Six Months Ended",
                            "september" => "Nine Months Ended",
                            _ => "Year Ended",
                        }
                    );
                } else if cell.text.eq_ignore_ascii_case("three months ended") {
                    cell.text = format!("Three Months Ended {date}");
                }
            }
        }
    }
    tables
}

fn extract(html: &str, source: &str) -> Result<Vec<crate::Observation>> {
    use chrono::{Datelike, NaiveDate};

    let date = regex::Regex::new(
        r"(?i)for the periods ended ((?:March|June|September|December) (?:30|31)), (20\d{2}) and (20\d{2})",
    )?;
    let mut observations = Vec::new();
    for (page, text) in crate::presentation::pages(html)?.iter().enumerate() {
        let Some(date) = date.captures(text) else {
            continue;
        };
        if !text.contains("Three Months Ended Year to Date") {
            continue;
        }
        for values in crate::presentation::values(text, r"NAREIT FFO per share\s*\(diluted\)")?
            .into_iter()
            .filter(|values| values.len() == 4)
        {
            for (column, value) in values.into_iter().enumerate() {
                let end = NaiveDate::parse_from_str(
                    &format!("{} {}", &date[1], &date[2 + column % 2]),
                    "%B %d %Y",
                )?;
                observations.push(crate::Observation {
                    published_on: None,
                    metric: Metric::Ffo,
                    start: NaiveDate::from_ymd_opt(
                        end.year(),
                        if column < 2 { end.month() - 2 } else { 1 },
                        1,
                    )
                    .unwrap(),
                    end,
                    label: "NAREIT FFO per share (diluted)".into(),
                    value,
                    unit: Unit::PerShare,
                    scale: 1,
                    source: source.to_owned(),
                    location: format!("presentation page {}, column {}", page + 1, column + 1),
                });
            }
        }
    }
    Ok(observations)
}

fn debt(html: &scraper::Html, as_of: chrono::NaiveDate) -> anyhow::Result<crate::debt::Debt> {
    crate::debt::tables(
        html,
        as_of,
        &[crate::debt::Rule {
            intro: "on notes payable and the unsecured credit facility",
            ..crate::debt::Rule::new(3, 1000)
        }],
        None,
    )
}
