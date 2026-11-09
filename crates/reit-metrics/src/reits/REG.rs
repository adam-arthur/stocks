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
    Ok(company)
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
    for table in &mut tables {
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
