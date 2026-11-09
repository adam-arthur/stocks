use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "DRH",
        1298946,
        &["drhc.com", "www.drhc.com", "investor.drhc.com"],
        vec![
            (
                Metric::Ffo,
                Unit::Total,
                r"^ffo available to common stock(?: and unit)?\s*holders$",
                1000,
            ),
            (Metric::Ffo, Unit::Total, r"^ffo$", 1000),
        ],
        None,
    ))?;
    company.tables = crate::table::joined_parentheses;
    company.debt = Some(debt);
    company.ownership = Some(crate::ownership::Rules {
        units: &[(
            "us-gaap:UnitsOfPartnershipInterestAmount",
            &["drh:UnaffiliatedThirdPartiesMember"],
        )],
        exchange_pattern: Some(
            r"(?i)common OP units for cash per unit.{0,160}common stock on a one\s*-for-one basis",
        ),
        prose: Some(ownership_prose),
        ..Default::default()
    });
    Ok(company)
}

fn debt(html: &scraper::Html, as_of: chrono::NaiveDate) -> anyhow::Result<crate::debt::Debt> {
    crate::debt::bullets(
        html,
        as_of,
        "Weighted-Average Interest Rate",
        (
            r"^(Term [123] Loan|Unsecured term loan|Mortgage loan|Senior unsecured credit facility)$",
            3,
            2,
        ),
        ("Total debt", &[]),
    )
}

fn ownership_prose(
    text: &str,
    source: &str,
    filed: Option<chrono::NaiveDate>,
    result: &mut std::collections::BTreeMap<chrono::NaiveDate, crate::ownership::Ownership>,
) -> anyhow::Result<()> {
    use crate::ownership::{Count, date, report_date};
    use regex::Regex;
    for matched in Regex::new(r"(?i)As of (?<date>[A-Za-z]+ \d{1,2}\s*, 20\d{2}), there were (?<count>[\d,]+) common OP units held by unaffiliated third parties")?.captures_iter(text) {
                if let Some(end) = date(&matched["date"]) {
                    result.entry(end).or_default().outside_op_units.get_or_insert(Count {
                        value: matched["count"].replace(',', "").parse()?,
                        source: source.into(), evidence: matched[0].into(), filed,
                    });
                }
            }
    if let Some(end) = report_date(text)
                && let Some(matched) = Regex::new(r"(?i)currently owns, either directly or indirectly, all of the limited partnership units of (?:the|our) Operating Partnership")?.find(text)
            {
                let count = Count { value: 0.0, source: source.into(), evidence: matched.as_str().into(), filed };
                result.entry(end).or_default().outside_op_units.get_or_insert(count.clone());
                result.entry(end).or_default().outside_op_unit_share_equivalents.get_or_insert(count);
            }

    Ok(())
}
