use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "OPI",
        1456772,
        &[
            "opireit.com",
            "www.opireit.com",
            "govreit.com",
            "www.govreit.com",
        ],
        vec![(
            Metric::Ffo,
            Unit::PerShare,
            r"^ffo(?: available for common shareholders)?(?: \(basic and diluted\))?$",
            1,
        )],
        None,
    ))?;
    company.rules[0].section = Some(regex::Regex::new(r"(?i)^per common share amounts")?);
    company.affo_support = crate::AffoSupport::Unavailable;
    company.extra_observations = Some(extract);
    company.tables = crate::table::joined_parentheses;
    company.debt = Some(debt);
    Ok(company)
}

fn extract(html: &str, source: &str) -> Result<Vec<crate::Observation>> {
    use chrono::{Datelike, NaiveDate};

    let section =
        regex::Regex::new(r"(?i)per common share (?:data|amounts)\s*\(basic and diluted\):")?;
    let dates = regex::Regex::new(r"\b(3|6|9|12)/(30|31)/(20\d{2})\b")?;
    let partial = regex::Regex::new(
        r"(?i)period from ([A-Z]+) (\d{1,2}) through ([A-Z]+) (\d{1,2}), (20\d{2})",
    )?;
    let mut observations = Vec::new();
    for (page, text) in crate::presentation::pages(html)?.iter().enumerate() {
        let Some(section) = section.find(text) else {
            continue;
        };
        let header = &text[..section.start()];
        if !header.to_lowercase().contains("for the three months ended") {
            continue;
        }
        let mut periods = partial
            .captures_iter(header)
            .filter_map(|date| {
                Some((
                    NaiveDate::parse_from_str(
                        &format!("{} {} {}", &date[1], &date[2], &date[5]),
                        "%B %d %Y",
                    )
                    .ok()?,
                    NaiveDate::parse_from_str(
                        &format!("{} {} {}", &date[3], &date[4], &date[5]),
                        "%B %d %Y",
                    )
                    .ok()?,
                ))
            })
            .collect::<Vec<_>>();
        periods.extend(
            dates
                .captures_iter(header)
                .filter_map(|date| {
                    let end = NaiveDate::parse_from_str(&date[0], "%m/%d/%Y").ok()?;
                    Some((
                        NaiveDate::from_ymd_opt(end.year(), end.month() - 2, 1)?,
                        end,
                    ))
                })
                .take(if !periods.is_empty() {
                    4
                } else if [
                    "for the six months ended",
                    "for the nine months ended",
                    "for the year ended",
                ]
                .iter()
                .any(|duration| header.to_lowercase().contains(duration))
                {
                    7
                } else {
                    5
                }),
        );
        if periods.len() == 7 {
            for (start, end) in &mut periods[5..] {
                *start = NaiveDate::from_ymd_opt(end.year(), 1, 1).unwrap();
            }
        }
        for (metric, unit, label, text, scale) in [(
            Metric::Ffo,
            Unit::PerShare,
            "FFO",
            &text[section.end()..],
            1,
        )] {
            let Some(values) = crate::presentation::values(text, label)?
                .into_iter()
                .next()
                .filter(|values| values.len() == periods.len())
            else {
                continue;
            };
            for (column, ((start, end), value)) in periods.iter().copied().zip(values).enumerate() {
                observations.push(crate::Observation {
                    published_on: None,
                    metric,
                    start,
                    end,
                    label: if unit == Unit::PerShare {
                        format!("{label} per common share (basic and diluted)")
                    } else {
                        label.into()
                    },
                    value,
                    unit,
                    scale,
                    source: source.to_owned(),
                    location: format!("presentation page {}, column {}", page + 1, column + 1),
                });
            }
        }
    }
    Ok(observations)
}

fn debt(html: &scraper::Html, as_of: chrono::NaiveDate) -> anyhow::Result<crate::debt::Debt> {
    if chrono::Datelike::month(&as_of) != 12 {
        let debt = crate::debt::tables(
            html,
            as_of,
            &[crate::debt::Rule {
                anchor: "",
                intro: "our debt maturities (other than our revolving credit facility)",
                ..crate::debt::Rule::new(0, 1000)
            }],
            None,
        )?;
        let debt = crate::debt::add_payments(
            debt,
            &crate::debt::instrument_payments(
                &crate::debt::rows(html, as_of, "Debtor-in-possession term loan")?,
                (r"^Secured revolving credit facility, due in", 1, 0),
            )?,
        )?;
        anyhow::ensure!(
            crate::debt::scalar(html, "us-gaap:DebtInstrumentCarryingAmount", as_of)?
                == Some(i64::try_from(debt.total_debt_principal)?),
            "OPI principal does not reconcile"
        );
        return Ok(debt);
    }
    crate::debt::tables(html, as_of, &[crate::debt::Rule::new(0, 1000)], None)
}
