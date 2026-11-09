use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "HIW",
        921082,
        &[],
        vec![
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^funds from operations available for common stockholders per share$",
                1,
            ),
            (
                Metric::Ffo,
                Unit::Total,
                r"^funds from operations available for common stockholders$",
                1000,
            ),
        ],
        None,
    ))?;
    company.filings = &["10-Q", "10-K"];
    company.derive = Some(|documents| crate::derive::fourth_quarters("HIW", documents));
    company.tables = crate::table::joined_parentheses;
    company.debt = Some(debt);
    company.ownership = Some(crate::ownership::Rules {
        units: &[(
            "hiw:CommonUnitsOfPartnershipNotOwned",
            &["srt:SubsidiariesMember"],
        )],
        exchange_pattern: Some(
            r"(?i)all of the Common Units not owned by the Company are redeemable on a one\s*-for-one basis for shares of its Common Stock",
        ),
        prose: Some(ownership_prose),
        ..Default::default()
    });
    Ok(company)
}

fn debt(html: &scraper::Html, as_of: chrono::NaiveDate) -> anyhow::Result<crate::debt::Debt> {
    use anyhow::Context;
    // Aggregate secured/unsecured balances and selected loan changes cannot replace scheduled principal payments by year.
    crate::debt::tables(html, as_of, &[crate::debt::Rule::new(0, 1000)], None).context(
        "HIW needs a complete principal repayment schedule, including mortgage amortization",
    )
}

fn ownership_prose(
    text: &str,
    source: &str,
    filed: Option<chrono::NaiveDate>,
    result: &mut std::collections::BTreeMap<chrono::NaiveDate, crate::ownership::Ownership>,
) -> anyhow::Result<()> {
    use crate::ownership::{Count, date};
    use regex::Regex;
    for matched in Regex::new(r"(?i)capitalization \(in thousands, except per share amounts\): (?<current>[A-Za-z]+ \d{1,2}\s*, 20\d{2}) (?<prior>[A-Za-z]+ \d{1,2}\s*, 20\d{2}).{0,800}Common Units outstanding \(not owned by the Company\) (?<count>[\d,]+) (?<previous>[\d,]+)")?.captures_iter(text) {
                for (period, count) in [("current", "count"), ("prior", "previous")] {
                    if let Some(end) = date(&matched[period]) {
                        result.entry(end).or_default().outside_op_units.get_or_insert(Count {
                            value: matched[count].replace(',', "").parse::<f64>()? * 1000.0,
                            source: source.into(), evidence: matched[0].into(), filed,
                        });
                    }
                }
            }

    Ok(())
}
