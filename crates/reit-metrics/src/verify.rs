use std::path::Path;

use anyhow::{Result, ensure};
use chrono::NaiveDate;
use serde::Deserialize;

use crate::{Metric, Unit, companies::company, extract_file, source::Sources};

#[derive(Deserialize)]
struct Case {
    symbol: String,
    source: String,
    count: usize,
    expected: Vec<Expected>,
}

#[derive(Deserialize)]
struct Expected {
    metric: Metric,
    start: NaiveDate,
    end: NaiveDate,
    value: f64,
    unit: Unit,
    scale: u64,
}

pub async fn verify(cases: &Path, cache: &Path) -> Result<usize> {
    let cases: Vec<Case> = serde_json::from_slice(&std::fs::read(cases)?)?;
    ensure!(!cases.is_empty(), "no verification cases");
    let sources = Sources::new(cache)?;
    for case in &cases {
        let path = sources
            .download(&company(&case.symbol)?, &case.source)
            .await?;
        let actual = extract_file(&case.symbol, &path, &case.source)?;
        ensure!(
            actual.observations.len() == case.count,
            "{}: expected {} observations, got {}",
            case.source,
            case.count,
            actual.observations.len()
        );
        for expected in &case.expected {
            ensure!(
                actual
                    .observations
                    .iter()
                    .any(|value| value.metric == expected.metric
                        && value.start == expected.start
                        && value.end == expected.end
                        && value.value == expected.value
                        && value.unit == expected.unit
                        && value.scale == expected.scale),
                "{}: missing expected {:?} {} = {}",
                case.source,
                expected.metric,
                expected.end,
                expected.value
            );
        }
        eprintln!("{}: verified {}", case.symbol, case.source);
    }
    Ok(cases.len())
}
