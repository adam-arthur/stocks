use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "AHRT",
        1569187,
        &[],
        vec![
            (
                Metric::Ffo,
                Unit::Total,
                r"^ffo (?:available|attributable) to common stockholders and op unitholders$",
                1000,
            ),
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^ffo (?:available|attributable) to common stockholders and op unitholders per diluted share and unit$",
                1,
            ),
        ],
        None,
    ))?;
    company.affo_support = crate::AffoSupport::Reported;
    company.extra_observations = Some(extract);
    company.debt = Some(|html, as_of| {
        crate::debt::tables(
            html,
            as_of,
            &[crate::debt::Rule {
                anchor: "",
                heading: "Amount Due",
                intro: "during each of the next five years and thereafter",
                ..crate::debt::Rule::new(0, 1000)
            }],
            None,
        )
    });
    Ok(company)
}

fn extract(html: &str, source: &str) -> Result<Vec<crate::Observation>> {
    use chrono::{Datelike, NaiveDate};

    let dates = regex::Regex::new(r"\b(?:3/31|6/30|9/30|12/31)/20\d{2}\b")?;
    let mut result = Vec::new();
    for (page, text) in crate::presentation::pages(html)?.into_iter().enumerate() {
        let lower = text.to_lowercase();
        let Some((_, data)) = lower.split_once("three months ended") else {
            continue;
        };
        if !lower.contains("in thousands") || !lower.contains("affo per diluted share") {
            continue;
        }
        let Some(end) = dates
            .find(data)
            .and_then(|m| NaiveDate::parse_from_str(m.as_str(), "%m/%d/%Y").ok())
        else {
            continue;
        };
        for (label, unit, scale) in [
            ("AFFO", Unit::Total, 1000),
            ("AFFO per Diluted Share", Unit::PerShare, 1),
        ] {
            if let Some(value) = crate::presentation::values(data, label)?
                .first()
                .and_then(|values| values.first())
                .copied()
            {
                result.push(crate::Observation {
                    published_on: None,
                    metric: Metric::Affo,
                    start: NaiveDate::from_ymd_opt(end.year(), end.month() - 2, 1).unwrap(),
                    end,
                    label: label.into(),
                    value,
                    unit,
                    scale,
                    source: source.into(),
                    location: format!("presentation page {}, first quarterly column", page + 1),
                });
            }
        }
    }
    Ok(result)
}
