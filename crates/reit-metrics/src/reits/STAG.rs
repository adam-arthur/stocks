use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "STAG",
        1479094,
        &["www.stagindustrial.com", "ir.stagindustrial.com"],
        vec![
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^(?:ffo|funds from operations) per share\s*/\s*unit\s*[-–—]\s*diluted$",
                1,
            ),
            (
                Metric::Ffo,
                Unit::Total,
                r"^funds from operations attributable to common stockholders and unit holders$",
                1000,
            ),
        ],
        None,
    ))?;
    company.debt = Some(debt);
    company.ownership = Some(crate::ownership::Rules {
        units: &[(
            "us-gaap:LimitedPartnersCapitalAccountUnitsOutstanding",
            &["stag:OtherNoncontrollingCommonUnitsMember"],
        )],
        exchange_pattern: Some(
            r"(?i)common units.{0,350}(?:for a share|shares) of common stock on a one\s*-for-one basis",
        ),
        exchange_source: Some(
            "https://www.sec.gov/Archives/edgar/data/1479094/000147909416000006/stag-123115x10k.htm",
        ),
        tables: Some(ownership_tables),
        ..Default::default()
    });
    Ok(company)
}

fn debt(html: &scraper::Html, as_of: chrono::NaiveDate) -> anyhow::Result<crate::debt::Debt> {
    crate::debt::tables(html, as_of, &[crate::debt::Rule::new(0, 1000)], None)
}

fn ownership_tables(
    html: &str,
    source: &str,
    filed: Option<chrono::NaiveDate>,
    result: &mut std::collections::BTreeMap<chrono::NaiveDate, crate::ownership::Ownership>,
) -> anyhow::Result<()> {
    use crate::ownership::{Count, date};
    for table in crate::table::tables(html) {
        let Some(column) = table
            .rows
            .iter()
            .flatten()
            .find(|cell| {
                matches!(
                    cell.text.as_str(),
                    "Other Common Units" | "Other Noncontrolling Common Units"
                )
            })
            .map(|cell| cell.column)
        else {
            continue;
        };
        for row in &table.rows {
            let Some(end) = row
                .iter()
                .find_map(|cell| cell.text.strip_prefix("Balance at ").and_then(date))
            else {
                continue;
            };
            if let Some(value) = row
                .iter()
                .find(|cell| cell.covers(column))
                .and_then(|cell| cell.text.replace(',', "").parse::<f64>().ok())
            {
                result
                    .entry(end)
                    .or_default()
                    .outside_op_units
                    .get_or_insert(Count {
                        value,
                        source: source.into(),
                        evidence: format!("Other Common Units; balance at {end}: {value}"),
                        filed,
                    });
            }
        }
    }
    Ok(())
}
