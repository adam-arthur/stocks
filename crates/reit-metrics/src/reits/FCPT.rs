use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "FCPT",
        1650132,
        &["investors.fcpt.com"],
        vec![
            (Metric::Ffo, Unit::PerShare, r"^ffo per diluted share$", 1),
            (Metric::Ffo, Unit::Total, r"^funds from operations$", 1000),
        ],
        None,
    ))?;
    company.tables = crate::table::joined_parentheses;
    company.extra_observations = Some(extract);
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
    let title =
        regex::Regex::new(r"(?i)FCPT Announces (First|Second|Third|Fourth) Quarter (20\d{2})")?;
    let Some(title) = title.captures(&text) else {
        return Ok(Vec::new());
    };
    let value = regex::Regex::new(
        r"(?i)NAREIT-defined FFO per diluted share for the (first|second|third|fourth) quarter was \$(\d+\.\d+)",
    )?;
    let Some(value) = value
        .captures(&text)
        .filter(|value| value[1].eq_ignore_ascii_case(&title[1]))
    else {
        return Ok(Vec::new());
    };
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
    Ok(vec![crate::Observation {
        metric: Metric::Ffo,
        start,
        end: start
            .checked_add_months(chrono::Months::new(3))
            .unwrap()
            .pred_opt()
            .unwrap(),
        label: "NAREIT-defined FFO per diluted share".into(),
        value: value[2].parse()?,
        unit: Unit::PerShare,
        scale: 1,
        source: source.into(),
        location: "earnings release FFO highlights".into(),
    }])
}
