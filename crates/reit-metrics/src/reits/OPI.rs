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
                .take(if periods.is_empty() { 5 } else { 4 }),
        );
        let Some(values) = crate::presentation::values(&text[section.end()..], "FFO")?
            .into_iter()
            .next()
            .filter(|values| values.len() == periods.len())
        else {
            continue;
        };
        for (column, ((start, end), value)) in periods.into_iter().zip(values).enumerate() {
            observations.push(crate::Observation {
                metric: Metric::Ffo,
                start,
                end,
                label: "FFO per common share (basic and diluted)".into(),
                value,
                unit: Unit::PerShare,
                scale: 1,
                source: source.to_owned(),
                location: format!("presentation page {}, column {}", page + 1, column + 1),
            });
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
