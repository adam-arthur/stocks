use crate::{Extraction, Observation, Unit};
use chrono::{Datelike, NaiveDate};

pub(crate) fn fourth_quarters(symbol: &str, documents: &[Extraction]) -> Vec<Extraction> {
    let mut totals = std::collections::BTreeMap::<_, Vec<_>>::new();
    for document in documents
        .iter()
        .filter(|document| document.symbol == symbol)
    {
        for value in document.observations.iter().filter(|value| {
            value.unit == Unit::Total
                && value.start == NaiveDate::from_ymd_opt(value.end.year(), 1, 1).unwrap()
                && matches!((value.end.month(), value.end.day()), (9, 30) | (12, 31))
        }) {
            totals
                .entry((value.metric, value.start, value.end))
                .or_default()
                .push((document, value));
        }
    }
    for values in totals.values_mut() {
        values.sort_by_key(|(document, value)| {
            (
                document.report_period != Some(value.end),
                document.filed_on.unwrap_or(NaiveDate::MAX),
                &value.source,
            )
        });
    }
    totals
        .iter()
        .filter(|((metric, _, end), _)| {
            end.month() == 12
                && !documents
                    .iter()
                    .filter(|document| document.symbol == symbol)
                    .flat_map(|document| &document.observations)
                    .any(|value| {
                        value.metric == *metric
                            && value.unit == Unit::Total
                            && value.end == *end
                            && value.start == NaiveDate::from_ymd_opt(end.year(), 10, 1).unwrap()
                    })
        })
        .map(|((metric, start, end), annuals)| {
            let (document, annual) = annuals[0];
            let mut result = Extraction {
                report_period: Some(*end),
                observations: Vec::new(),
                ..document.clone()
            };
            let Some(nine_months) = totals.get(&(
                *metric,
                *start,
                NaiveDate::from_ymd_opt(end.year(), 9, 30).unwrap(),
            )) else {
                return result;
            };
            let (quarterly, nine_month) = nine_months[0];
            if [annuals, nine_months].iter().any(|values| {
                values.iter().any(|(_, value)| {
                    !value.value.is_finite()
                        || value.scale == 0
                        || value.value * value.scale as f64
                            != values[0].1.value * values[0].1.scale as f64
                })
            }) {
                return result;
            }
            let value =
                annual.value * annual.scale as f64 - nine_month.value * nine_month.scale as f64;
            if !value.is_finite() {
                return result;
            }
            result.filed_on = document
                .filed_on
                .zip(quarterly.filed_on)
                .map(|(annual, quarterly)| annual.max(quarterly));
            result.observations.push(Observation {
                published_on: annual
                    .published_on
                    .zip(nine_month.published_on)
                    .map(|(a, b)| a.max(b)),
                start: NaiveDate::from_ymd_opt(end.year(), 10, 1).unwrap(),
                label: format!("Calculated Q4 {}", annual.label),
                value,
                scale: 1,
                location: format!(
                    "calculated: annual {} ({}) minus nine-month {} ({})",
                    annual.source, annual.location, nine_month.source, nine_month.location
                ),
                ..annual.clone()
            });
            result
        })
        .collect()
}
