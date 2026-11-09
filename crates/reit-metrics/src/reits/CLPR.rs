use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "CLPR",
        1649096,
        &["www.clipperrealty.com", "investors.clipperrealty.com"],
        vec![(Metric::Ffo, Unit::Total, r"^ffo$", 1000)],
        None,
    ))?;
    company.tables = crate::table::joined_parentheses;
    company.debt = Some(debt);
    company.ownership = Some(crate::ownership::Rules {
        exchange_pattern: Some(
            r"(?i)Class B LLC units, together with the special voting shares, are convertible into common shares of the Company on a one\s*-for-one basis",
        ),
        exchange_source: Some(
            "https://www.sec.gov/Archives/edgar/data/1649096/000143774917005646/clpr20170329_10k.htm",
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
    use crate::ownership::{Count, report_date};
    use regex::Regex;
    if let Some(end) = report_date(text) {
        if let Some(matched) = Regex::new(r"(?i)The continuing investors (?:own|owned) an aggregate amount of ([\d,]+) Class B LLC units")?.captures(text) {
                    result.entry(end).or_default().outside_op_units = Some(Count {
                        value: matched[1].replace(',', "").parse()?, source: source.into(), evidence: matched[0].into(), filed,
                    });
                } else if let Some(matched) = Regex::new(r"(?i)The effect of the conversion of the (?<count>[\d,]+) Class B LLC units outstanding.{0,800}\((?:dollars? )?in thousands, except per share amounts\)")?.captures(text) {
                    result.entry(end).or_default().outside_op_units.get_or_insert(Count {
                        value: matched["count"].replace(',', "").parse::<f64>()? * 1000.0, source: source.into(), evidence: matched[0].into(), filed,
                    });
                }
    }

    Ok(())
}
