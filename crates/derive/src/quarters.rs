use std::collections::{BTreeMap, BTreeSet};

use chrono::NaiveDate;
use serde_json::{Value, json};

use crate::financials::{Financials, Observation};

pub fn group(data: &Financials) -> Vec<Value> {
    let durations = data
        .values()
        .flatten()
        .filter(|o| o.start.is_some())
        .collect::<Vec<_>>();
    let mut annuals = BTreeMap::<NaiveDate, &Observation>::new();
    for o in &durations {
        if o.annual {
            let entry = annuals.entry(o.start.unwrap()).or_insert(o);
            if (o.end, o.filed) > (entry.end, entry.filed) {
                *entry = o;
            }
        }
    }
    let mut starts = annuals.keys().copied().collect::<BTreeSet<_>>();
    for start in annuals
        .values()
        .filter_map(|o| o.end.succ_opt())
        .collect::<BTreeSet<_>>()
    {
        if !annuals
            .iter()
            .any(|(begin, o)| *begin <= start && start <= o.end)
            && !starts
                .iter()
                .any(|begin| (start - *begin).num_days().abs() < 300)
        {
            starts.insert(start);
        }
    }
    let mut result = Vec::new();
    for year_start in starts.iter().copied() {
        let mut ends = BTreeMap::<u8, BTreeMap<NaiveDate, usize>>::new();
        for o in &durations {
            if o.start.unwrap() < year_start
                || starts
                    .range((
                        std::ops::Bound::Excluded(year_start),
                        std::ops::Bound::Unbounded,
                    ))
                    .next()
                    .is_some_and(|next| o.end >= *next)
                || !(o.start == Some(year_start)
                    || quarter((o.end - o.start.unwrap()).num_days()) == Some(1))
            {
                continue;
            }
            if let Some(q) = quarter((o.end - year_start).num_days()) {
                *ends.entry(q).or_default().entry(o.end).or_default() += 1;
            }
        }
        let ends = ends
            .into_iter()
            .map(|(q, dates)| {
                (
                    q,
                    dates
                        .into_iter()
                        .max_by_key(|(end, count)| (*count, *end))
                        .unwrap()
                        .0,
                )
            })
            .collect::<BTreeMap<_, _>>();
        for (&q, &end) in &ends {
            let start = if q == 1 {
                Some(year_start)
            } else {
                ends.get(&(q - 1))
                    .and_then(|end| end.succ_opt())
                    .or_else(|| {
                        durations
                            .iter()
                            .filter(|o| {
                                o.end == end
                                    && o.start.unwrap() >= year_start
                                    && quarter((o.end - o.start.unwrap()).num_days()) == Some(1)
                            })
                            .max_by_key(|o| o.filed)
                            .and_then(|o| o.start)
                    })
            };
            let mut record =
                json!({"fiscal_year_start": year_start, "quarter": q, "start": start, "end": end});
            for (metric, observations) in data {
                record[metric] = json!(if observations.iter().any(|o| o.start.is_some()) {
                    start.and_then(|start| value(observations, metric, year_start, start, end))
                } else {
                    observations
                        .iter()
                        .filter(|o| o.end == end)
                        .max_by_key(|o| o.filed)
                        .map(|o| o.value)
                });
            }
            result.push(record);
        }
    }
    result.sort_by_key(|record| record["end"].as_str().unwrap().to_owned());
    result
}

fn quarter(days: i64) -> Option<u8> {
    match days {
        70..=120 => Some(1),
        150..=210 => Some(2),
        240..=300 => Some(3),
        350..=380 => Some(4),
        _ => None,
    }
}

fn value(
    observations: &[Observation],
    metric: &str,
    year_start: NaiveDate,
    start: NaiveDate,
    end: NaiveDate,
) -> Option<f64> {
    if quarter((end - start).num_days()) != Some(1) {
        return None;
    }
    if let Some(o) = observations
        .iter()
        .filter(|o| o.start == Some(start) && o.end == end)
        .max_by_key(|o| o.filed)
    {
        return Some(o.value);
    }
    if metric == "diluted_earnings_per_share" || start == year_start {
        return None;
    }
    let current = observations
        .iter()
        .filter(|o| o.start == Some(year_start) && o.end == end)
        .max_by_key(|o| o.filed)?;
    let prior = observations
        .iter()
        .filter(|o| {
            o.start == Some(year_start)
                && o.end.succ_opt() == Some(start)
                && o.filed <= current.filed
                && o.unit == current.unit
        })
        .max_by_key(|o| o.filed)?;
    Some(current.value - prior.value)
}
