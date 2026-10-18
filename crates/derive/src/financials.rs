use std::collections::BTreeMap;

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Serialize)]
pub struct Observation {
    pub value: f64,
    pub start: Option<NaiveDate>,
    pub end: NaiveDate,
    pub filed: NaiveDate,
    pub unit: String,
    #[serde(skip)]
    pub annual: bool,
    #[serde(skip)]
    priority: usize,
}

#[derive(Deserialize)]
struct Fact {
    val: f64,
    start: Option<NaiveDate>,
    end: NaiveDate,
    filed: NaiveDate,
    form: String,
    fp: Option<String>,
}

pub type Financials = BTreeMap<String, Vec<Observation>>;

pub fn extract(facts: &Value, as_of: NaiveDate) -> Financials {
    let mut result = Financials::new();
    for (metric, unit, duration, tags) in [
        (
            "revenue",
            "USD",
            true,
            &[
                "RevenueFromContractWithCustomerExcludingAssessedTax",
                "RevenueFromContractWithCustomerIncludingAssessedTax",
                "Revenues",
                "SalesRevenueNet",
            ][..],
        ),
        (
            "earnings",
            "USD",
            true,
            &["NetIncomeLoss", "ProfitLoss"][..],
        ),
        (
            "common_shareholder_earnings",
            "USD",
            true,
            &["NetIncomeLossAvailableToCommonStockholdersBasic"][..],
        ),
        (
            "diluted_earnings_per_share",
            "USD/shares",
            true,
            &["EarningsPerShareDiluted"][..],
        ),
        (
            "cash_and_equivalents",
            "USD",
            false,
            &["CashAndCashEquivalentsAtCarryingValue"][..],
        ),
        (
            "shareholder_equity",
            "USD",
            false,
            &["StockholdersEquity"][..],
        ),
        (
            "debt_due_within_one_year",
            "USD",
            false,
            &["DebtCurrent"][..],
        ),
        (
            "finance_leases_due_within_one_year",
            "USD",
            false,
            &[
                "FinanceLeaseLiabilityCurrent",
                "CapitalLeaseObligationsCurrent",
            ][..],
        ),
        ("long_term_debt", "USD", false, &["LongTermDebt"][..]),
        (
            "long_term_debt_due_within_one_year",
            "USD",
            false,
            &["LongTermDebtCurrent"][..],
        ),
        (
            "debt_due_after_one_year",
            "USD",
            false,
            &["LongTermDebtNoncurrent"][..],
        ),
        (
            "short_term_debt",
            "USD",
            false,
            &["ShortTermBorrowings"][..],
        ),
        ("commercial_paper", "USD", false, &["CommercialPaper"][..]),
        (
            "other_short_term_debt",
            "USD",
            false,
            &["OtherShortTermBorrowings"][..],
        ),
        (
            "preferred_stock_equity",
            "USD",
            false,
            &[
                "PreferredStockIncludingAdditionalPaidInCapitalNetOfDiscount",
                "PreferredStockIncludingAdditionalPaidInCapital",
            ][..],
        ),
        (
            "temporary_shareholder_equity",
            "USD",
            false,
            &["TemporaryEquityCarryingAmountAttributableToParent"][..],
        ),
        (
            "preferred_shares_outstanding",
            "shares",
            false,
            &["PreferredStockSharesOutstanding"][..],
        ),
        (
            "shares_outstanding",
            "shares",
            false,
            &[
                "dei:EntityCommonStockSharesOutstanding",
                "CommonStockSharesOutstanding",
            ][..],
        ),
    ] {
        let mut observations = BTreeMap::<_, Option<Observation>>::new();
        for (priority, tag) in tags.iter().enumerate() {
            let (namespace, name) = tag.split_once(':').unwrap_or(("us-gaap", tag));
            for fact in facts["facts"][namespace][name]["units"][unit]
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
                    || fact.start.is_some() != duration
                    || fact.start.is_some_and(|start| start > fact.end)
                    || !matches!(
                        fact.form.as_str(),
                        "10-K"
                            | "10-K/A"
                            | "10-Q"
                            | "10-Q/A"
                            | "20-F"
                            | "20-F/A"
                            | "40-F"
                            | "40-F/A"
                    )
                {
                    continue;
                }
                let key = (fact.start, fact.end, fact.filed);
                match observations.entry(key) {
                    std::collections::btree_map::Entry::Vacant(entry) => {
                        entry.insert(Some(Observation {
                            value: fact.val,
                            start: fact.start,
                            end: fact.end,
                            filed: fact.filed,
                            unit: unit.into(),
                            annual: fact.fp.as_deref() == Some("FY")
                                && matches!(
                                    fact.form.as_str(),
                                    "10-K" | "10-K/A" | "20-F" | "20-F/A" | "40-F" | "40-F/A"
                                )
                                && fact.start.is_some_and(|start| {
                                    (350..=380).contains(&(fact.end - start).num_days())
                                })
                                && (0..=180).contains(&(fact.filed - fact.end).num_days()),
                            priority,
                        }));
                    }
                    std::collections::btree_map::Entry::Occupied(mut entry) => {
                        if entry
                            .get()
                            .as_ref()
                            .is_some_and(|old| old.priority == priority && old.value != fact.val)
                        {
                            entry.insert(None);
                        }
                    }
                }
            }
        }
        result.insert(
            metric.into(),
            observations.into_values().flatten().collect(),
        );
    }
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
            "short_term_debt",
            vec![vec![
                ("commercial_paper", 1.0),
                ("other_short_term_debt", 1.0),
            ]],
        ),
        (
            "total_debt",
            vec![
                vec![
                    ("debt_due_within_one_year", 1.0),
                    ("debt_due_after_one_year", 1.0),
                    ("finance_leases_due_within_one_year", -1.0),
                ],
                vec![("long_term_debt", 1.0), ("short_term_debt", 1.0)],
                vec![
                    ("long_term_debt_due_within_one_year", 1.0),
                    ("debt_due_after_one_year", 1.0),
                    ("short_term_debt", 1.0),
                ],
            ],
        ),
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
            vec![vec![
                ("shareholder_equity", 1.0),
                ("preferred_stock_equity", -1.0),
            ]],
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
            if at(&result, metric, end, filed).is_some() {
                continue;
            }
            for formula in &formulas {
                if let Some(parts) = formula
                    .iter()
                    .map(|(name, weight)| at(&result, name, end, filed).map(|o| (o, *weight)))
                    .collect::<Option<Vec<_>>>()
                {
                    derived.push(combine(&parts, None, end));
                    break;
                }
            }
        }
        result.entry(metric.into()).or_default().extend(derived);
    }
    result
}

fn combine(parts: &[(&Observation, f64)], start: Option<NaiveDate>, end: NaiveDate) -> Observation {
    Observation {
        value: parts.iter().map(|(o, weight)| o.value * weight).sum(),
        start,
        end,
        filed: parts.iter().map(|(o, _)| o.filed).max().unwrap(),
        unit: parts[0].0.unit.clone(),
        annual: false,
        priority: usize::MAX,
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
