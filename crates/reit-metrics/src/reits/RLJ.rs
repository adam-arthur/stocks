use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "RLJ",
        1511337,
        &[
            "www.rljlodgingtrust.com",
            "rljlodgingtrust.com",
            "investor.rljlodgingtrust.com",
        ],
        vec![(Metric::Ffo, Unit::Total, r"^ffo$", 1000)],
        None,
    ))?;
    company.tables = crate::table::joined_parentheses;
    company.debt = Some(debt);
    company.ownership = Some(crate::ownership::Rules {
        units: &[(
            "us-gaap:PartnersCapitalAccountUnits",
            &["us-gaap:LimitedPartnerMember"],
        )],
        exchange_pattern: Some(
            r"(?i)(?:OP units|outstanding units held by the limited partners).{0,250}for a like number of common shares",
        ),
        exchange_source: Some(
            "https://www.sec.gov/Archives/edgar/data/1511337/000151133717000027/rlj-20161231x10k.htm",
        ),
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
    use crate::ownership::{Count, date, report_date};
    use regex::Regex;
    for matched in Regex::new(r"(?i)As of (?<date>[A-Za-z]+ \d{1,2}\s*, 20\d{2})\s*, the limited partners owned (?<count>[\d,]+) OP units")?.captures_iter(text) {
                if let Some(end) = date(&matched["date"]) {
                    result.entry(end).or_default().outside_op_units.get_or_insert(Count {
                        value: matched["count"].replace(',', "").parse()?, source: source.into(), evidence: matched[0].into(), filed,
                    });
                }
            }
    if let Some(end) = report_date(text) {
        for pattern in [
            r"(?i)After the redemption, (?<count>[\d,]+) operating partnership units remain outstanding",
            r"(?i)(?<count>[\d,]+) outstanding OP units (?:are|were) held by the limited partners",
            r"(?i)The limited partners own (?<count>[\d,]+) OP units",
        ] {
            if let Some(matched) = Regex::new(pattern)?.captures(text) {
                result
                    .entry(end)
                    .or_default()
                    .outside_op_units
                    .get_or_insert(Count {
                        value: matched["count"].replace(',', "").parse()?,
                        source: source.into(),
                        evidence: matched[0].into(),
                        filed,
                    });
            }
        }
    }
    let common = Regex::new(
        r"(?i)Common shares of beneficial interest,.{0,140}shares authorized; (?<count>[\d,]+) and [\d,]+ shares issued and outstanding at (?<date>[A-Za-z]+ \d{1,2}\s*, 20\d{2})",
    )?;
    for matched in Regex::new(r#"(?i)As of (?<date>[A-Za-z]+ \d{1,2}\s*, 20\d{2})\s*, there were (?<count>[\d,]+) units of limited partnership interest in the Operating Partnership \(["“]OP units["”]\) outstanding"#)?.captures_iter(text) {
                if let Some(end) = date(&matched["date"])
                    && let Some(shares) = common.captures_iter(text).find(|shares| date(&shares["date"]) == Some(end))
                {
                    let value = matched["count"].replace(',', "").parse::<f64>()? - shares["count"].replace(',', "").parse::<f64>()?;
                    if value >= 0.0 {
                        result.entry(end).or_default().outside_op_units.get_or_insert(Count {
                            value, source: source.into(), evidence: format!("Total common OP units less company-owned units represented by common shares: {}; {}", &matched[0], &shares[0]), filed,
                        });
                    }
                }
            }

    Ok(())
}
