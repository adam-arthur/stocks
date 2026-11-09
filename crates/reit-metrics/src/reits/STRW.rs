use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "STRW",
        1782430,
        &["strawberryfieldsreit.com", "www.strawberryfieldsreit.com"],
        vec![
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^ffo per weighted average common share and op units$",
                1,
            ),
            (Metric::Ffo, Unit::Total, r"^funds from operations$", 1000),
        ],
        None,
    ))?;
    company.filings = &["8-K", "8-K/A", "10-Q", "10-Q/A", "10-K", "10-K/A"];
    company.tables = crate::table::joined_parentheses;
    company.derive = Some(|documents| crate::derive::fourth_quarters("STRW", documents));
    company.additional_items = Some(|items| items.split(',').any(|item| item.trim() == "7.01"));
    company.debt = Some(debt);
    company.ownership = Some(crate::ownership::Rules {
        units: &[(
            "us-gaap:LimitedPartnersCapitalAccountUnitsOutstanding",
            &["STRW:OperatingPartnershipAgreementMember"],
        )],
        prose: Some(ownership_prose),
        ..Default::default()
    });
    Ok(company)
}

fn debt(html: &scraper::Html, as_of: chrono::NaiveDate) -> anyhow::Result<crate::debt::Debt> {
    crate::debt::tables(html, as_of, &[crate::debt::Rule::new(0, 1000)], None)
}

fn ownership_prose(
    text: &str,
    source: &str,
    filed: Option<chrono::NaiveDate>,
    result: &mut std::collections::BTreeMap<chrono::NaiveDate, crate::ownership::Ownership>,
) -> anyhow::Result<()> {
    use crate::ownership::{Count, date};
    use regex::Regex;
    for matched in Regex::new(r"(?i)reserved a total of (?<count>[\d,]+) shares of common stock that may be issued, at the Company.s option, upon redemption of the OP units outstanding as of (?<date>[A-Za-z]+ \d{1,2}, 20\d{2})")?.captures_iter(text) {
                if let Some(end) = date(&matched["date"]) {
                    result.entry(end).or_default().outside_op_unit_share_equivalents = Some(Count {
                        value: matched["count"].replace(',', "").parse()?, source: source.into(), evidence: matched[0].into(), filed,
                    });
                }
            }

    Ok(())
}
