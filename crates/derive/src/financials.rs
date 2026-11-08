use std::collections::BTreeMap;

use chrono::NaiveDate;

pub use ::financials::{Financials, Observation};

pub fn calculate(result: &mut Financials) {
    for (metric, evidence) in [
        ("preferred_stock_equity", "preferred_shares_outstanding"),
        (
            "redeemable_preferred_stock_value",
            "temporary_shareholder_equity",
        ),
    ] {
        let zeros = result[evidence]
            .iter()
            .filter(|o| o.value == 0.0)
            .map(|o| Observation {
                unit: "USD".into(),
                ..o.clone()
            })
            .collect::<Vec<_>>();
        result.entry(metric.into()).or_default().extend(zeros);
    }
    for (metric, formulas) in [
        (
            "preferred_stock_value",
            vec![vec![
                ("preferred_stock_equity", 1.0),
                ("redeemable_preferred_stock_value", 1.0),
            ]],
        ),
        (
            "total_debt_including_preferred_stock",
            vec![vec![("total_debt", 1.0), ("preferred_stock_value", 1.0)]],
        ),
        (
            "common_shareholder_equity",
            vec![
                vec![
                    ("shareholder_equity", 1.0),
                    ("preferred_stock_equity", -1.0),
                ],
                vec![("shareholder_equity", 1.0)],
            ],
        ),
    ] {
        let dates = result
            .values()
            .flatten()
            .filter(|o| o.start.is_none())
            .map(|o| (o.end, o.filed))
            .collect::<std::collections::BTreeSet<_>>();
        let mut derived = Vec::new();
        for (end, filed) in dates {
            if at(result, metric, end, filed).is_some() {
                continue;
            }
            for formula in &formulas {
                if let Some(parts) = formula
                    .iter()
                    .map(|(name, weight)| at(result, name, end, filed).map(|o| (o, *weight)))
                    .collect::<Option<Vec<_>>>()
                {
                    derived.push(combine(&parts, None, end));
                    break;
                }
            }
        }
        result.entry(metric.into()).or_default().extend(derived);
    }
}

fn combine(parts: &[(&Observation, f64)], start: Option<NaiveDate>, end: NaiveDate) -> Observation {
    Observation {
        value: parts.iter().map(|(o, weight)| o.value * weight).sum(),
        start,
        end,
        filed: parts.iter().map(|(o, _)| o.filed).max().unwrap(),
        unit: parts[0].0.unit.clone(),
        annual: false,
    }
}

fn at<'a>(
    data: &'a Financials,
    metric: &str,
    end: NaiveDate,
    filed: NaiveDate,
) -> Option<&'a Observation> {
    data.get(metric)?
        .iter()
        .filter(|o| o.end == end && o.filed <= filed)
        .max_by_key(|o| o.filed)
}

pub fn latest<'a>(data: &'a Financials, metric: &str, date: NaiveDate) -> Option<&'a Observation> {
    data.get(metric)?
        .iter()
        .filter(|o| o.end <= date && o.filed <= date)
        .max_by_key(|o| (o.end, o.filed, o.start))
}

pub fn trailing(data: &Financials, metric: &str, date: NaiveDate) -> Option<Observation> {
    let mut periods = BTreeMap::new();
    for o in data
        .get(metric)?
        .iter()
        .filter(|o| o.end <= date && o.filed <= date && o.start.is_some())
    {
        let entry = periods.entry((o.start.unwrap(), o.end)).or_insert(o);
        if o.filed > entry.filed {
            *entry = o;
        }
    }
    let end = periods.values().map(|o| o.end).max()?;
    if let Some(annual) = periods
        .values()
        .find(|o| o.end == end && (350..=380).contains(&(o.end - o.start.unwrap()).num_days()))
    {
        return Some((*annual).clone());
    }
    let mut quarters = Vec::new();
    let mut cursor = end;
    for _ in 0..4 {
        let Some(quarter) = periods.values().find(|o| {
            o.end == cursor && (70..=110).contains(&(o.end - o.start.unwrap()).num_days())
        }) else {
            break;
        };
        quarters.push((*quarter, 1.0));
        cursor = quarter.start?.pred_opt()?;
    }
    if quarters.len() == 4 && (350..=380).contains(&(end - cursor).num_days()) {
        return Some(combine(&quarters, cursor.succ_opt(), end));
    }
    for current in periods.values().filter(|o| o.end == end) {
        for annual in periods.values().filter(|o| {
            o.end.succ_opt() == current.start
                && (350..=380).contains(&(o.end - o.start.unwrap()).num_days())
        }) {
            if let Some(prior) = periods.values().find(|o| {
                o.start == annual.start
                    && (350..=380).contains(&(end - o.end).num_days())
                    && ((current.end - current.start.unwrap()).num_days()
                        - (o.end - o.start.unwrap()).num_days())
                    .abs()
                        <= 7
            }) {
                return Some(combine(
                    &[(current, 1.0), (annual, 1.0), (prior, -1.0)],
                    prior.end.succ_opt(),
                    end,
                ));
            }
        }
    }
    None
}
