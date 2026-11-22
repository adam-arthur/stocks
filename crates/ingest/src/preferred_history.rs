use std::collections::{BTreeMap, BTreeSet};

use alpaca_client::{AlpacaClient, preferred_exchange::exchanges};
use anyhow::Result;
use chrono::{Duration, NaiveDate};
use serde_json::Value;

use crate::{CorporateAction, Store, corporate_actions::fetch_actions, prices};

pub(crate) async fn update(
    client: &AlpacaClient,
    store: &Store,
    symbol: &str,
    start: NaiveDate,
    end: NaiveDate,
    actions: &mut [CorporateAction],
) -> Result<()> {
    for action in actions.iter_mut().filter(|a| a.kind == "stock_mergers") {
        action.fields.remove("predecessor_dividends");
    }
    let records = actions
        .iter()
        .map(serde_json::to_value)
        .collect::<Result<Vec<_>, _>>()?;
    let candidates: Vec<_> = exchanges(&records, symbol, start, end).collect();
    let [(exchange, date)] = candidates.as_slice() else {
        return Ok(());
    };
    let (Some(old), Some(cusip), Some(new_cusip)) = (
        exchange["acquiree_symbol"].as_str(),
        exchange["acquiree_cusip"].as_str(),
        exchange["acquirer_cusip"].as_str(),
    ) else {
        return Ok(());
    };
    if old.is_empty()
        || old == symbol
        || cusip.len() != 9
        || new_cusip.len() != 9
        || cusip == new_cusip
        || exchange["acquirer_rate"].as_f64() != Some(1.0)
        || exchange["acquiree_rate"].as_f64() != Some(1.0)
    {
        return Ok(());
    }
    let Some(first_payment) = records
        .iter()
        .filter(|a| {
            a["type"] == "cash_dividends"
                && a["symbol"] == symbol
                && a["cusip"] == new_cusip
                && a["special"] == false
        })
        .min_by_key(|a| a["ex_date"].as_str())
    else {
        return Ok(());
    };
    let before = prices::fetch(
        client,
        &[old.into(), symbol.into()],
        start.and_hms_opt(0, 0, 0).unwrap().and_utc(),
        date.and_hms_opt(0, 0, 0).unwrap().and_utc() - Duration::seconds(1),
        Some("-"),
    )
    .await?;
    let retained: BTreeMap<_, _> = store
        .prices(symbol)?
        .range(start..*date)
        .map(|(&d, &p)| (d, p))
        .collect();
    if before.get(symbol).is_some_and(|p| !p.is_empty()) || before.get(old) != Some(&retained) {
        return Ok(());
    }
    let mut payments = BTreeMap::new();
    let mut ids = BTreeSet::new();
    for action in fetch_actions(client, ("cusips", cusip), start, end).await? {
        if action.id == exchange["id"].as_str().unwrap_or_default()
            || action.effective_date() < start
        {
            continue;
        }
        if action.kind != "cash_dividends" {
            return Ok(());
        }
        let id = action.id.clone();
        let entry = serde_json::to_value(action)?;
        let Some(ex) = entry["ex_date"]
            .as_str()
            .and_then(|d| d.parse::<NaiveDate>().ok())
        else {
            return Ok(());
        };
        if ex >= *date
            || entry["symbol"] != old
            || entry["cusip"] != cusip
            || entry["rate"].as_f64().is_none_or(|v| v <= 0.0)
            || entry["special"].as_bool().is_none()
            || entry.get("payable_date").is_some_and(|d| {
                d.as_str()
                    .is_none_or(|date| date.parse::<NaiveDate>().is_err())
            })
            || records.iter().any(|a| {
                a["type"] == "cash_dividends"
                    && (a["id"] == entry["id"] || a["ex_date"] == entry["ex_date"])
                    && a != &entry
            })
        {
            return Ok(());
        }
        if payments.get(&ex) == Some(&entry) {
            continue;
        }
        if !ids.insert(id) || payments.insert(ex, entry).is_some() {
            return Ok(());
        }
    }
    if continuous(&payments, start, first_payment) {
        let merger = actions
            .iter_mut()
            .find(|a| Some(a.id.as_str()) == exchange["id"].as_str())
            .unwrap();
        merger.fields.insert(
            "predecessor_dividends".into(),
            Value::Array(payments.into_values().collect()),
        );
    }
    Ok(())
}

fn continuous(payments: &BTreeMap<NaiveDate, Value>, start: NaiveDate, successor: &Value) -> bool {
    let regular: Vec<_> = payments
        .iter()
        .filter(|(_, a)| a["special"] == false)
        .collect();
    let Some((last, payment)) = regular.last() else {
        return false;
    };
    let Some(next) = successor["ex_date"]
        .as_str()
        .and_then(|d| d.parse::<NaiveDate>().ok())
    else {
        return false;
    };
    regular.len() >= 3
        && payment["rate"].as_f64() == successor["rate"].as_f64()
        && alpaca_client::preferred_exchange::CADENCES
            .into_iter()
            .any(|(days, _)| {
                days.contains(&(next - **last).num_days())
                    && (*regular[0].0 - start).num_days() <= *days.end()
                    && regular
                        .windows(2)
                        .all(|w| days.contains(&(*w[1].0 - *w[0].0).num_days()))
            })
}
