use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "FCPT",
        1650132,
        &["investors.fcpt.com"],
        vec![
            (
                Metric::Affo,
                Unit::PerShare,
                r#"^affo per diluted share$"#,
                1,
            ),
            (
                Metric::Affo,
                Unit::Total,
                r#"^adjusted funds from operations(?: \(affo\))?$"#,
                1000,
            ),
            (Metric::Ffo, Unit::PerShare, r"^ffo per diluted share$", 1),
            (Metric::Ffo, Unit::Total, r"^funds from operations$", 1000),
        ],
        None,
    ))?;
    company.tables = crate::table::joined_parentheses;
    company.extra_observations = Some(extract);
    company.affo_support = crate::AffoSupport::Reported;
    company.debt = Some(debt);
    Ok(company)
}

fn extract(html: &str, source: &str) -> Result<Vec<crate::Observation>> {
    let text = crate::table::normalize(
        &scraper::Html::parse_document(html)
            .root_element()
            .text()
            .collect::<Vec<_>>()
            .join(" "),
    );
    let title = regex::Regex::new(
        r"(?i)FCPT Announces (First|Second|Third|Fourth) Quarter (?:and (?:Full|Fiscal) Year )?(20\d{2})",
    )?;
    let Some(title) = title.captures(&text) else {
        return Ok(Vec::new());
    };
    let value = regex::Regex::new(
        r"(?i)(NAREIT-defined FFO|AFFO) per diluted share for the (first|second|third|fourth) quarter was \$(\d+\.\d+)",
    )?;
    let start = chrono::NaiveDate::from_ymd_opt(
        title[2].parse()?,
        match title[1].to_lowercase().as_str() {
            "first" => 1,
            "second" => 4,
            "third" => 7,
            _ => 10,
        },
        1,
    )
    .unwrap();
    value
        .captures_iter(&text)
        .filter(|value| value[2].eq_ignore_ascii_case(&title[1]))
        .map(|value| {
            Ok(crate::Observation {
                published_on: None,
                metric: if value[1].eq_ignore_ascii_case("AFFO") {
                    Metric::Affo
                } else {
                    Metric::Ffo
                },
                start,
                end: start
                    .checked_add_months(chrono::Months::new(3))
                    .unwrap()
                    .pred_opt()
                    .unwrap(),
                label: format!("{} per diluted share", &value[1]),
                value: value[3].parse()?,
                unit: Unit::PerShare,
                scale: 1,
                source: source.into(),
                location: "earnings release highlights".into(),
            })
        })
        .collect()
}

fn debt(html: &scraper::Html, as_of: chrono::NaiveDate) -> anyhow::Result<crate::debt::Debt> {
    crate::debt::tables(html, as_of, &[crate::debt::Rule::new(0, 1000)], None)
}
