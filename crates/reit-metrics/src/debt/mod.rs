use std::collections::{BTreeMap, BTreeSet, HashMap};

use anyhow::{Context, Result, ensure};
use chrono::{Datelike, NaiveDate};
use regex::Regex;
use scraper::{ElementRef, Html, Selector};
use serde::{Deserialize, Serialize};

use crate::{companies::company, source::Sources, table::normalize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Debt {
    pub as_of: NaiveDate,
    pub total_debt_principal: u64,
    pub maturities: BTreeMap<i32, u64>,
    pub thereafter: u64,
}

pub(crate) struct Rule {
    pub anchor: &'static str,
    pub column: usize,
    pub heading: &'static str,
    pub intro: &'static str,
    pub row: Option<(&'static str, usize)>,
    pub sum_columns: usize,
    pub scale: u64,
}

impl Rule {
    pub(crate) fn new(column: usize, scale: u64) -> Self {
        Self {
            anchor: "us-gaap:LongTermDebtMaturitiesRepaymentsOfPrincipalInNextTwelveMonths",
            column,
            scale,
            heading: "",
            intro: "",
            row: None,
            sum_columns: 0,
        }
    }
}

pub async fn fetch(symbol: &str, sources: &Sources) -> Result<Debt> {
    let company = company(symbol)?;
    let mut filings = sources
        .sec
        .earnings_filings(
            company.cik,
            chrono::Utc::now().date_naive() - chrono::Days::new(730),
        )
        .await?;
    filings.retain(|filing| matches!(filing.form.as_str(), "10-K" | "10-K/A" | "10-Q" | "10-Q/A"));
    filings.sort_by_key(|filing| std::cmp::Reverse((filing.filed_on, filing.accession.clone())));
    let mut last_error = None;
    let mut amended: Option<Debt> = None;
    for filing in filings {
        let mut snapshots = Vec::new();
        for url in sources
            .filing_urls(&company, &filing.accession, &filing.form)
            .await?
        {
            if !url.to_ascii_lowercase().ends_with(".pdf") {
                match extract(symbol, &sources.get(&company, &url).await?) {
                    Ok(debt) => {
                        ensure!(
                            debt.as_of <= filing.filed_on,
                            "debt schedule is dated after its filing"
                        );
                        if !snapshots.contains(&debt) {
                            snapshots.push(debt);
                        }
                    }
                    Err(error) => {
                        last_error.get_or_insert(error);
                    }
                }
            }
        }
        ensure!(snapshots.len() <= 1, "conflicting debt snapshots");
        if let Some(debt) = snapshots.pop() {
            if filing.form.ends_with("/A") {
                if amended
                    .as_ref()
                    .is_none_or(|previous| debt.as_of > previous.as_of)
                {
                    amended = Some(debt);
                }
            } else {
                return Ok(amended
                    .filter(|previous| previous.as_of >= debt.as_of)
                    .unwrap_or(debt));
            }
        }
    }
    if let Some(debt) = amended {
        return Ok(debt);
    }
    Err(last_error
        .unwrap_or_else(|| anyhow::anyhow!("filings contain no extractable debt schedule")))
}

pub fn extract(symbol: &str, document: &str) -> Result<Debt> {
    let html = Html::parse_document(document);
    let named = Selector::parse("[name]").unwrap();
    let period = html
        .select(&named)
        .find(|node| node.value().attr("name") == Some("dei:DocumentPeriodEndDate"))
        .context("missing report period")?;
    let context_id = period
        .value()
        .attr("contextref")
        .context("missing report context")?;
    let context = html
        .select(&Selector::parse("[id]").unwrap())
        .find(|node| node.value().attr("id") == Some(context_id))
        .context("missing report context")?;
    let as_of: NaiveDate = context
        .descendants()
        .filter_map(ElementRef::wrap)
        .find(|node| node.value().name().rsplit(':').next() == Some("enddate"))
        .context("missing report end date")?
        .text()
        .collect::<String>()
        .parse()?;
    ensure!(
        html.select(&named)
            .any(|node| node.value().attr("name") == Some("dei:DocumentType")
                && matches!(
                    normalize(&node.text().collect::<String>()).as_str(),
                    "10-K" | "10-K/A" | "10-Q" | "10-Q/A"
                )),
        "debt extraction requires an annual or quarterly filing"
    );
    company(symbol)?
        .debt
        .context("missing company debt parser")?(&html, as_of)
}

pub(crate) fn tables(
    html: &Html,
    as_of: NaiveDate,
    rules: &[Rule],
    principal: Option<u64>,
) -> Result<Debt> {
    let named = Selector::parse("[name]").unwrap();
    let current_contexts = current_contexts(html, as_of);
    let mut prefixes = HashMap::new();
    let mut preceding = String::new();
    for node in html.root_element().descendants() {
        if node
            .value()
            .as_element()
            .is_some_and(|element| element.name() == "table")
        {
            prefixes.insert(node.id(), normalize(&preceding));
        }
        if let Some(text) = node.value().as_text() {
            preceding.push_str(text);
            preceding.push(' ');
            if preceding.len() > 4000 {
                let mut start = preceding.len() - 2000;
                while !preceding.is_char_boundary(start) {
                    start += 1;
                }
                preceding.drain(..start);
            }
        }
    }
    let mut parts = Vec::new();
    for rule in rules {
        let mut matches = Vec::new();
        for table in html
            .select(&Selector::parse("table").unwrap())
            .filter(|table| {
                table
                    .select(&Selector::parse("table").unwrap())
                    .next()
                    .is_none()
            })
        {
            let facts: Vec<_> = table
                .select(&named)
                .filter(|node| {
                    node.value().attr("name").is_some_and(|name| {
                        name.contains("DebtMaturit")
                            || name.contains("DebtAndCapitalLeaseObligationsMaturit")
                    })
                })
                .collect();
            if !facts.is_empty()
                && !facts.iter().any(|fact| {
                    fact.value()
                        .attr("contextref")
                        .is_some_and(|context| current_contexts.contains(context))
                })
            {
                continue;
            }
            if (rule.anchor.is_empty()
                || table
                    .select(&named)
                    .any(|node| node.value().attr("name") == Some(rule.anchor)))
                && normalize(&table.text().collect::<Vec<_>>().join(" ")).contains(rule.heading)
                && prefixes
                    .get(&table.id())
                    .is_some_and(|text| text.contains(rule.intro))
                && let Some(values) = schedule(table, rule, as_of, principal)?
                && !matches.contains(&values)
            {
                matches.push(values);
            }
        }
        ensure!(
            matches.len() == 1,
            "expected one complete debt table for {}, found {}",
            rule.anchor,
            matches.len()
        );
        parts.push(matches.remove(0));
    }
    let mut maturities = BTreeMap::new();
    let mut thereafter = 0u64;
    for (years, remaining) in parts {
        for (year, amount) in years {
            let value = maturities.entry(year).or_insert(0u64);
            *value = value.checked_add(amount).context("debt amount overflow")?;
        }
        thereafter = thereafter
            .checked_add(remaining)
            .context("debt amount overflow")?;
    }
    let total_debt_principal = maturities
        .values()
        .try_fold(thereafter, |sum, value| sum.checked_add(*value))
        .context("debt total overflow")?;
    Ok(Debt {
        as_of,
        total_debt_principal,
        maturities,
        thereafter,
    })
}

fn schedule(
    table: ElementRef<'_>,
    rule: &Rule,
    as_of: NaiveDate,
    principal: Option<u64>,
) -> Result<Option<(BTreeMap<i32, u64>, u64)>> {
    let heading_year = Regex::new(r"(?:^|\s)(20\d{2})(?:\D|$)")?;
    let year = Regex::new(r"^(?:december 31,?\s+)?(20\d{2})(?:\D|$)")?;
    let mut maturities = BTreeMap::new();
    let mut thereafter = None;
    let mut total = None;
    let mut seen = BTreeSet::new();
    let mut remainder_seen = false;
    let mut adjusted = false;
    for row in table.select(&Selector::parse("tr").unwrap()) {
        let mut cells: Vec<_> = row
            .select(&Selector::parse("td, th").unwrap())
            .map(|cell| normalize(&cell.text().collect::<String>()))
            .filter(|text| !text.is_empty())
            .collect();
        if cells.first().is_some_and(|cell| {
            cell.starts_with("For the fiscal years") || cell.starts_with("For the remaining")
        }) && cells.get(1).is_some_and(|cell| year.is_match(cell))
        {
            cells.remove(0);
        }
        let Some(label) = cells.first() else { continue };
        if let Some((target, offset)) = rule.row {
            if !label.starts_with(target) {
                continue;
            }
            let values: Vec<_> = cells
                .iter()
                .skip(1)
                .filter_map(|cell| amount(cell, rule.scale))
                .collect();
            if values.len() < offset + 6 {
                continue;
            }
            let headings = table
                .select(&Selector::parse("tr").unwrap())
                .find_map(|row| {
                    let years: Vec<i32> = row
                        .select(&Selector::parse("td, th").unwrap())
                        .filter_map(|cell| {
                            let text = normalize(&cell.text().collect::<String>());
                            let matched = heading_year.captures(&text)?;
                            let value = matched[1].parse().ok()?;
                            (value >= as_of.year()
                                && value <= as_of.year() + 5
                                && !text.to_lowercase().contains("thereafter")
                                && !text.contains('+'))
                            .then_some(value)
                        })
                        .collect();
                    (years.len() >= 5 && years.windows(2).all(|pair| pair[1] == pair[0] + 1))
                        .then_some(years)
                })
                .context("missing maturity year headings")?;
            ensure!(
                values.len() > offset + headings.len(),
                "missing horizontal debt amounts"
            );
            let years = headings
                .iter()
                .enumerate()
                .map(|(index, year)| (*year, values[offset + index]))
                .collect::<BTreeMap<_, _>>();
            let remaining = values[offset + headings.len()];
            let sum = years.values().sum::<u64>() + remaining;
            if let Some(total) = if offset == 1 {
                values.first()
            } else {
                values.get(headings.len() + 1)
            } {
                ensure!(
                    sum.abs_diff(*total) <= rule.scale * 6,
                    "horizontal debt schedule does not reconcile"
                );
            }
            return Ok(Some((years, remaining)));
        }
        let label = label.to_ascii_lowercase();
        adjusted |= label.contains("unamortized")
            || label.contains("issuance costs")
            || label.contains("discount")
            || label.contains("premium")
            || label.contains("non-cash")
            || label.contains("financing costs");
        let bucket = year
            .captures(&label)
            .map(|matched| matched[1].parse::<i32>())
            .transpose()?
            .or_else(|| {
                (as_of.month() != 12
                    && label.contains(&as_of.year().to_string())
                    && (label.contains("remainder")
                        || label.contains("remaining")
                        || label.contains("december 31")))
                .then_some(as_of.year())
            });
        let remaining = label.contains("thereafter")
            || label.starts_with("subsequent years")
            || label == "later"
            || label == "later years"
            || label == "beyond 5 years"
            || label == "over 5 years";
        let is_total = label == "total"
            || label.starts_with("total (")
            || label == "subtotal"
            || label == "$"
            || label == "principal amount"
            || label == "total minimum payments"
            || label.starts_with("total principal")
            || label == "total debt"
            || label == "total debt principle payments"
            || label.starts_with("total maturities")
            || label.starts_with("total scheduled principal")
            || label.starts_with("total long-term debt - face value");
        if bucket.is_none() && !remaining && !is_total {
            continue;
        }
        let amounts: Vec<_> = cells
            .iter()
            .skip(1)
            .filter_map(|cell| amount(cell, rule.scale))
            .collect();
        let value = if rule.sum_columns > 0 {
            if amounts.len() < rule.sum_columns {
                continue;
            }
            amounts[..rule.sum_columns]
                .iter()
                .try_fold(0u64, |sum, amount| sum.checked_add(*amount))
                .context("debt amount overflow")?
        } else {
            let Some(value) = if rule.column == usize::MAX {
                amounts.last()
            } else {
                amounts.get(rule.column)
            }
            .copied() else {
                continue;
            };
            value
        };
        if let Some(year) = bucket.filter(|_| !remaining) {
            if year < as_of.year() || (year == as_of.year() && as_of.month() == 12) {
                return Ok(None);
            }
            ensure!(seen.insert(year), "duplicate maturity year {year}");
            if year <= as_of.year() + 5 {
                maturities.insert(year, value);
            } else {
                thereafter = Some(
                    thereafter
                        .unwrap_or(0u64)
                        .checked_add(value)
                        .context("debt amount overflow")?,
                );
            }
        } else if remaining {
            ensure!(!remainder_seen, "duplicate thereafter bucket");
            remainder_seen = true;
            thereafter = Some(
                thereafter
                    .unwrap_or(0u64)
                    .checked_add(value)
                    .context("debt amount overflow")?,
            );
        } else if total.is_none() && !adjusted {
            total = Some(value);
        }
    }
    if maturities.len() < 5
        || maturities
            .keys()
            .copied()
            .ne(*maturities.keys().next().unwrap_or(&0)
                ..=*maturities.keys().next_back().unwrap_or(&0))
        || !maturities.contains_key(&(as_of.year() + 1))
    {
        return Ok(None);
    }
    let thereafter = match thereafter {
        Some(value) => value,
        None if total == Some(maturities.values().sum()) => 0,
        None if principal.is_some() => principal
            .unwrap()
            .checked_sub(maturities.values().sum())
            .context("scheduled repayments exceed principal")?,
        None => return Ok(None),
    };
    let sum = maturities
        .values()
        .try_fold(thereafter, |sum, value| sum.checked_add(*value))
        .context("debt total overflow")?;
    if let Some(total) = total {
        ensure!(
            sum.abs_diff(total) <= rule.scale * 6,
            "maturity schedule does not reconcile: {sum} versus {total}"
        );
    }
    Ok(Some((maturities, thereafter)))
}

pub(crate) fn amount(text: &str, scale: u64) -> Option<u64> {
    let text = text.trim().trim_start_matches('$').trim();
    if matches!(text, "—" | "–" | "-" | "-0-") {
        return Some(0);
    }
    let value = text.replace([',', ' '], "").parse::<f64>().ok()? * scale as f64;
    (value.is_finite() && value >= 0.0 && value < u64::MAX as f64).then(|| value.round() as u64)
}

pub(crate) fn scalar(html: &Html, tag: &str, end: NaiveDate) -> Result<Option<i64>> {
    let mut values = BTreeSet::new();
    for fact in html
        .select(&Selector::parse("[name][contextref]").unwrap())
        .filter(|fact| fact.value().attr("name") == Some(tag))
    {
        let context = html
            .select(&Selector::parse("[id]").unwrap())
            .find(|node| node.value().attr("id") == fact.value().attr("contextref"))
            .context("missing debt fact context")?;
        if context
            .descendants()
            .filter_map(ElementRef::wrap)
            .any(|node| {
                matches!(
                    node.value().name().rsplit(':').next(),
                    Some("explicitmember" | "typedmember")
                )
            })
        {
            continue;
        }
        if !context
            .descendants()
            .filter_map(ElementRef::wrap)
            .any(|node| {
                node.value().name().rsplit(':').next() == Some("instant")
                    && node.text().collect::<String>() == end.to_string()
            })
        {
            continue;
        }
        let unit = html
            .select(&Selector::parse("[id]").unwrap())
            .find(|node| node.value().attr("id") == fact.value().attr("unitref"))
            .context("missing debt fact unit")?;
        ensure!(
            normalize(&unit.text().collect::<String>()).ends_with(":USD"),
            "debt fact is not USD"
        );
        let scale = fact.value().attr("scale").unwrap_or("0").parse::<u32>()?;
        ensure!(scale <= 9, "invalid debt fact scale or sign");
        let value = if fact.value().attr("format").is_some_and(|format| {
            format.ends_with("fixed-zero")
                || format.ends_with("zerodash")
                || (format.ends_with("numwordsen")
                    && normalize(&fact.text().collect::<String>()) == "no")
        }) {
            0
        } else {
            amount(
                &normalize(&fact.text().collect::<String>()),
                10u64.pow(scale),
            )
            .context("invalid debt fact amount")?
        };
        values.insert(
            i64::try_from(value)?
                * if fact.value().attr("sign") == Some("-") {
                    -1
                } else {
                    1
                },
        );
    }
    ensure!(values.len() <= 1, "conflicting debt principal facts");
    Ok(values.first().copied())
}

pub(crate) fn payments(as_of: NaiveDate, payments: &[(i32, u64)]) -> Result<Debt> {
    let first = as_of.year() + i32::from(as_of.month() == 12);
    let mut debt = Debt {
        as_of,
        total_debt_principal: 0,
        maturities: (first..first + 5).map(|year| (year, 0)).collect(),
        thereafter: 0,
    };
    for (year, amount) in payments {
        ensure!(*year >= first, "debt includes past-due principal");
        let bucket = if *year < first + 5 {
            debt.maturities.get_mut(year).unwrap()
        } else {
            &mut debt.thereafter
        };
        *bucket = bucket
            .checked_add(*amount)
            .context("debt amount overflow")?;
        debt.total_debt_principal = debt
            .total_debt_principal
            .checked_add(*amount)
            .context("debt total overflow")?;
    }
    Ok(debt)
}

pub(crate) fn rows(html: &Html, as_of: NaiveDate, heading: &str) -> Result<Vec<Vec<String>>> {
    let current_contexts = current_contexts(html, as_of);
    let footnote = Regex::new(r"^\([1-9]\)$")?;
    let mut matches = Vec::new();
    for table in html
        .select(&Selector::parse("table").unwrap())
        .filter(|table| {
            table
                .select(&Selector::parse("table").unwrap())
                .next()
                .is_none()
                && normalize(&table.text().collect::<Vec<_>>().join(" ")).contains(heading)
                && table
                    .select(&Selector::parse("[contextref]").unwrap())
                    .any(|fact| {
                        fact.value()
                            .attr("contextref")
                            .is_some_and(|context| current_contexts.contains(context))
                    })
        })
    {
        let rows = table
            .select(&Selector::parse("tr").unwrap())
            .map(|row| {
                row.select(&Selector::parse("td, th").unwrap())
                    .filter(|cell| {
                        !footnote.is_match(&normalize(&cell.text().collect::<String>()))
                            || cell
                                .select(&Selector::parse("[name][contextref]").unwrap())
                                .next()
                                .is_some()
                    })
                    .map(|cell| normalize(&cell.text().collect::<String>()))
                    .filter(|cell| !cell.is_empty() && cell != "$" && cell != "%")
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        if !matches.contains(&rows) {
            matches.push(rows);
        }
    }
    ensure!(
        matches.len() == 1,
        "expected one debt instrument table for {heading}"
    );
    Ok(matches.pop().unwrap())
}

pub(crate) fn bullets(
    html: &Html,
    as_of: NaiveDate,
    heading: &str,
    columns: (&str, usize, usize),
    total: (&str, &[&str]),
) -> Result<Debt> {
    let rows = rows(html, as_of, heading)?;
    let payments = instrument_payments(&rows, columns)?;
    let mut reported = None;
    let mut adjustments = BTreeMap::new();
    for row in &rows {
        let Some(label) = row.first() else { continue };
        if label == total.0 {
            ensure!(reported.is_none(), "duplicate debt total");
            reported = row.iter().skip(1).find_map(|value| amount(value, 1000));
        } else if let Some(prefix) = total.1.iter().find(|prefix| label.starts_with(**prefix)) {
            ensure!(
                !adjustments.contains_key(prefix),
                "duplicate debt adjustment"
            );
            adjustments.insert(
                prefix,
                row.iter()
                    .skip(1)
                    .find_map(|value| amount(value.trim_matches(['(', ')', ' ']), 1000))
                    .context("missing debt adjustment")?,
            );
        }
    }
    ensure!(
        adjustments.len() == total.1.len(),
        "incomplete debt instrument table"
    );
    let debt = self::payments(as_of, &payments)?;
    ensure!(
        debt.total_debt_principal
            == adjustments.values().try_fold(
                reported.context("missing debt instrument total")?,
                |sum, value| sum.checked_add(*value).context("debt total overflow")
            )?,
        "instrument principal does not reconcile to reported debt"
    );
    Ok(debt)
}

pub(crate) fn add_payments(mut debt: Debt, payments: &[(i32, u64)]) -> Result<Debt> {
    for (year, amount) in payments {
        ensure!(
            *year
                >= *debt
                    .maturities
                    .keys()
                    .next()
                    .context("missing maturity years")?,
            "debt includes past-due principal"
        );
        let bucket = if *year > *debt.maturities.keys().next_back().unwrap() {
            &mut debt.thereafter
        } else {
            debt.maturities
                .get_mut(year)
                .context("missing maturity year")?
        };
        *bucket = bucket
            .checked_add(*amount)
            .context("debt amount overflow")?;
        debt.total_debt_principal = debt
            .total_debt_principal
            .checked_add(*amount)
            .context("debt total overflow")?;
    }
    Ok(debt)
}

pub(crate) fn instrument_payments(
    rows: &[Vec<String>],
    columns: (&str, usize, usize),
) -> Result<Vec<(i32, u64)>> {
    let instrument = Regex::new(columns.0)?;
    let year = Regex::new(r"\b(20\d{2})\b")?;
    let mut payments = Vec::new();
    let mut names = BTreeSet::new();
    for row in rows {
        let Some(label) = row.first().filter(|label| instrument.is_match(label)) else {
            continue;
        };
        ensure!(
            names.insert((label.clone(), row.get(columns.2).cloned())),
            "duplicate debt instrument"
        );
        let principal = row
            .get(columns.1)
            .and_then(|value| amount(value, 1000))
            .context("missing instrument principal")?;
        if principal == 0 {
            continue;
        }
        let years = year
            .captures_iter(row.get(columns.2).context("missing instrument maturity")?)
            .map(|matched| matched[1].parse::<i32>())
            .collect::<std::result::Result<Vec<_>, _>>()?;
        ensure!(years.len() == 1, "ambiguous instrument maturity");
        payments.push((years[0], principal));
    }
    ensure!(!names.is_empty(), "missing debt instruments");
    Ok(payments)
}

fn current_contexts(html: &Html, as_of: NaiveDate) -> BTreeSet<&str> {
    html.select(&Selector::parse("[id]").unwrap())
        .filter(|node| node.value().name().rsplit(':').next() == Some("context"))
        .filter(|context| {
            context
                .descendants()
                .filter_map(ElementRef::wrap)
                .any(|node| {
                    node.value().name().rsplit(':').next() == Some("instant")
                        && node.text().collect::<String>() == as_of.to_string()
                })
        })
        .filter_map(|node| node.value().attr("id"))
        .collect()
}
