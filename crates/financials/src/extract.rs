use std::collections::{BTreeMap, BTreeSet};

use chrono::NaiveDate;
use serde::Deserialize;
use serde_json::Value;

use crate::{Extraction, Financials, Issue, Observation, Rule, common, symbols};

#[derive(Deserialize)]
struct Fact {
    val: f64,
    start: Option<NaiveDate>,
    end: NaiveDate,
    filed: NaiveDate,
    form: String,
    fp: Option<String>,
}

type Period = (Option<NaiveDate>, NaiveDate, NaiveDate);

pub fn extract(symbol: &str, facts: &Value, as_of: NaiveDate) -> Extraction {
    extract_with_rules(facts, as_of, symbols::rules(symbol))
}

pub fn extract_with_rules(facts: &Value, as_of: NaiveDate, overrides: Vec<Rule>) -> Extraction {
    let mut result = Extraction {
        financials: Financials::new(),
        issues: Vec::new(),
    };
    for rule in common::rules().into_iter().chain(overrides) {
        let values = result.financials.entry(rule.metric.into()).or_default();
        values.retain(|o| !rule.includes(o.end));
        result
            .issues
            .retain(|i| i.metric != rule.metric || !rule.includes(i.end));
        let tags = rule
            .formulas
            .iter()
            .flatten()
            .map(|(tag, _)| *tag)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .map(|tag| (tag, observations(facts, &rule, tag, as_of)))
            .collect::<BTreeMap<_, _>>();
        let periods = tags
            .values()
            .flat_map(|values| values.keys().copied())
            .collect::<BTreeSet<_>>();
        for (start, end, filed) in periods {
            let mut selected = None;
            let mut conflict = false;
            for formula in &rule.formulas {
                if formula.is_empty() {
                    continue;
                }
                let Some(parts) = formula
                    .iter()
                    .map(|(tag, weight)| {
                        tags[tag]
                            .get(&(start, end, filed))
                            .map(|o| (o.as_ref(), weight))
                    })
                    .collect::<Option<Vec<_>>>()
                else {
                    continue;
                };
                if parts.iter().any(|(o, _)| o.is_none()) {
                    conflict = true;
                    break;
                }
                let value = parts
                    .iter()
                    .map(|(o, weight)| o.unwrap().value * *weight)
                    .sum::<f64>();
                if !value.is_finite()
                    || (matches!(rule.metric, "total_debt" | "cash_and_equivalents") && value < 0.0)
                {
                    conflict = true;
                    break;
                }
                selected = Some(Observation {
                    value,
                    ..parts[0].0.unwrap().clone()
                });
                break;
            }
            if conflict {
                values.retain(|o| o.start != start || o.end != end);
                result.issues.push(Issue {
                    metric: rule.metric.into(),
                    end,
                    problem: "conflicting or invalid facts",
                });
            } else if let Some(observation) = selected {
                result
                    .issues
                    .retain(|i| i.metric != rule.metric || i.end != end);
                values.push(observation);
            }
        }
        values.sort_by_key(|o| (o.start, o.end, o.filed));
    }
    if let Some(end) = result
        .financials
        .values()
        .flatten()
        .filter(|o| o.start.is_none() && o.unit == "USD")
        .map(|o| o.end)
        .max()
    {
        for metric in ["total_debt", "total_assets", "cash_and_equivalents"] {
            if !result.financials[metric].iter().any(|o| o.end == end)
                && !result
                    .issues
                    .iter()
                    .any(|i| i.metric == metric && i.end == end)
            {
                result.issues.push(Issue {
                    metric: metric.into(),
                    end,
                    problem: "no complete Company Facts mapping",
                });
            }
        }
    }
    result
}

fn observations(
    facts: &Value,
    rule: &Rule,
    tag: &str,
    as_of: NaiveDate,
) -> BTreeMap<Period, Option<Observation>> {
    let (namespace, name) = tag.split_once(':').unwrap_or(("us-gaap", tag));
    let mut result = BTreeMap::<Period, Option<Observation>>::new();
    for fact in facts["facts"][namespace][name]["units"][rule.unit]
        .as_array()
        .into_iter()
        .flatten()
    {
        let Ok(fact) = serde_json::from_value::<Fact>(fact.clone()) else {
            continue;
        };
        if !fact.val.is_finite()
            || fact.filed > as_of
            || fact.end > as_of
            || fact.start.is_some() != rule.duration
            || fact.start.is_some_and(|start| start > fact.end)
            || !rule.includes(fact.end)
            || !matches!(
                fact.form.as_str(),
                "10-K" | "10-K/A" | "10-Q" | "10-Q/A" | "20-F" | "20-F/A" | "40-F" | "40-F/A"
            )
        {
            continue;
        }
        let observation = Observation {
            value: fact.val,
            start: fact.start,
            end: fact.end,
            filed: fact.filed,
            unit: rule.unit.into(),
            annual: fact.fp.as_deref() == Some("FY")
                && matches!(
                    fact.form.as_str(),
                    "10-K" | "10-K/A" | "20-F" | "20-F/A" | "40-F" | "40-F/A"
                )
                && fact
                    .start
                    .is_some_and(|start| (350..=380).contains(&(fact.end - start).num_days()))
                && (0..=180).contains(&(fact.filed - fact.end).num_days()),
        };
        result
            .entry((fact.start, fact.end, fact.filed))
            .and_modify(|old| {
                if old.as_ref().is_some_and(|old| old.value != fact.val) {
                    *old = None;
                }
            })
            .or_insert(Some(observation));
    }
    result
}
