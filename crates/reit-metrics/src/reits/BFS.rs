use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "BFS",
        907254,
        &[
            "ir.saulcenters.com",
            "www.saulcenters.com",
            "saulcenters.com",
        ],
        vec![
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^(?:(?:basic and )?diluted )?ffo per share available to common stockholders and noncontrolling interests$",
                1,
            ),
            (
                Metric::Ffo,
                Unit::Total,
                r"^ffo available to common stockholders and noncontrolling interests$",
                1000,
            ),
        ],
        None,
    ))?;
    company.debt = Some(debt);
    company.ownership = Some(crate::ownership::Rules {
        exchange_pattern: Some(
            r"(?i)units are convertible into shares of.{0,180}Saul Centers. common stock.{0,70}one\s*-for-one basis",
        ),
        prose: Some(ownership_prose),
        ..Default::default()
    });
    Ok(company)
}

fn debt(html: &scraper::Html, as_of: chrono::NaiveDate) -> anyhow::Result<crate::debt::Debt> {
    crate::debt::tables(
        html,
        as_of,
        &[crate::debt::Rule {
            anchor: if chrono::Datelike::month(&as_of) == 12 {
                "us-gaap:LongTermDebtMaturitiesRepaymentsOfPrincipalInNextTwelveMonths"
            } else {
                "bfs:LongTermDebtMaturitiesRepaymentsOfPrincipalAndBalloonPaymentsRemainderOfFiscalYear"
            },
            ..crate::debt::Rule::new(usize::MAX, 1000)
        }],
        None,
    )
}

fn ownership_prose(
    text: &str,
    source: &str,
    filed: Option<chrono::NaiveDate>,
    result: &mut std::collections::BTreeMap<chrono::NaiveDate, crate::ownership::Ownership>,
) -> anyhow::Result<()> {
    use crate::ownership::{Count, date};
    use chrono::{Datelike, NaiveDate};
    use regex::Regex;
    let third_party = Regex::new(
        r"(?i)As of (?:each of )?(?<current>[A-Za-z]+ \d{1,2}\s*, 20\d{2})(?: and (?<prior>[A-Za-z]+ \d{1,2}\s*, 20\d{2}))?\s*, a third[ -]party investor (?:held|holds).{0,150}represented by (?:approximately )?(?<count>[\d,]+) convertible limited partnership units",
    )?;
    for pattern in [
        r"(?i)As of (?<date>[A-Za-z]+ \d{1,2}\s*, 20\d{2})\s*, the (?:B\. F\. Saul Company.{0,350}|Saul Organization) (?:holds a|hold an aggregate|held an aggregate).{0,100}represented by approximately (?<count>[\d.]+) (?<scale>million) convertible limited partnership units",
        r"(?i)The Saul Organization holds a [\d.]+\s*% limited partnership interest in the Operating Partnership represented by (?<count>[\d,]+) limited partnership units, as of (?<date>[A-Za-z]+ \d{1,2}\s*, 20\d{2})",
    ] {
        for matched in Regex::new(pattern)?.captures_iter(text) {
            let Some(end) = date(&matched["date"]) else {
                continue;
            };
            if filed.is_some_and(|filed| end > filed)
                || (matched.name("scale").is_some() && end.month() == 12)
            {
                continue;
            }
            let investor = third_party.captures_iter(text).find(|investor| {
                ["current", "prior"].iter().any(|field| {
                    investor.name(field).and_then(|value| date(value.as_str())) == Some(end)
                })
            });
            if investor.is_none() && end >= NaiveDate::from_ymd_opt(2021, 6, 30).unwrap() {
                continue;
            }
            result.entry(end).or_default().outside_op_units = Some(Count {
                value: matched["count"].replace(',', "").parse::<f64>()?
                    * if matched.name("scale").is_some() {
                        1_000_000.0
                    } else {
                        1.0
                    }
                    + investor
                        .as_ref()
                        .map(|value| value["count"].replace(',', "").parse::<f64>())
                        .transpose()?
                        .unwrap_or(0.0),
                source: source.into(),
                evidence: investor.as_ref().map_or_else(
                    || matched[0].to_owned(),
                    |investor| format!("{}; {}", &matched[0], &investor[0]),
                ),
                filed,
            });
        }
    }

    Ok(())
}
