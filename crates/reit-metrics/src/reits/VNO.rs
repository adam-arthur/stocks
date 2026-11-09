use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "VNO",
        899689,
        &["vno.com", "www.vno.com", "investors.vno.com"],
        vec![(
            Metric::Ffo,
            Unit::Total,
            r"^ffo attributable to common shareholders plus assumed conversions(?: \(non-gaap\))?$",
            1000,
        )],
        None,
    ))?;
    company.tables = tables;
    company.debt = Some(debt);
    company.ownership = Some(crate::ownership::Rules {
        units: &[(
            "us-gaap:TemporaryEquitySharesOutstanding",
            &["us-gaap:CapitalUnitClassAMember"],
        )],
        exchange_pattern: Some(
            r"(?i)(?:Vornado, at its option|we, at our option), may assume that obligation and pay the holder either cash or Vornado common shares on a one\s*-for-one basis",
        ),
        exchange_source: Some(
            "https://www.sec.gov/Archives/edgar/data/899689/000089968916000046/vno2015form10k.htm",
        ),
        prose: Some(ownership_prose),
        ..Default::default()
    });
    Ok(company)
}

pub(crate) fn tables(html: &str) -> Vec<crate::table::Table> {
    crate::table::tables(html)
        .into_iter()
        .filter(|table| {
            !table
                .rows
                .iter()
                .take(6)
                .flatten()
                .any(|cell| cell.text.to_lowercase().contains("financial highlights"))
        })
        .map(|mut table| {
            for cell in table.rows.iter_mut().flatten() {
                match cell.text.to_lowercase().as_str() {
                    "for the quarters" => cell.text = "Three Months Ended".into(),
                    "for the years" => cell.text = "Year Ended".into(),
                    _ => {}
                }
            }
            table
        })
        .collect()
}

fn debt(_html: &scraper::Html, _as_of: chrono::NaiveDate) -> anyhow::Result<crate::debt::Debt> {
    anyhow::bail!(
        "maturity schedule excludes overdue principal; cannot represent total debt with future-year buckets alone"
    )
}

fn ownership_prose(
    text: &str,
    source: &str,
    filed: Option<chrono::NaiveDate>,
    result: &mut std::collections::BTreeMap<chrono::NaiveDate, crate::ownership::Ownership>,
) -> anyhow::Result<()> {
    use crate::ownership::{Count, report_date};
    use regex::Regex;
    if let Some(end) = report_date(text)
                && let Some(matched) = Regex::new(r"(?i)Redeemable noncontrolling interests: Class A units\s*[-–—]\s*(?<count>[\d,]+) and [\d,]+ units outstanding")?.captures(text)
            {
                result.entry(end).or_default().outside_op_units.get_or_insert(Count {
                    value: matched["count"].replace(',', "").parse()?,
                    source: source.into(), evidence: matched[0].into(), filed,
                });
            }

    Ok(())
}
