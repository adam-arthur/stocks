use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "BHR",
        1574085,
        &["www.bhrreit.com", "bhrreit.com"],
        vec![(
            Metric::Ffo,
            Unit::Total,
            r"^ffo available to common stockholders and op unitholders$",
            1000,
        )],
        None,
    ))?;
    company.tables = crate::table::joined_parentheses;
    company.debt = Some(debt);
    company.ownership = Some(crate::ownership::Rules {
        prose: Some(ownership_prose),
        ..Default::default()
    });
    Ok(company)
}

fn debt(html: &scraper::Html, as_of: chrono::NaiveDate) -> anyhow::Result<crate::debt::Debt> {
    use anyhow::Context;
    if chrono::Datelike::month(&as_of) != 12 {
        // Allocate the reviewed interest-only loans to current maturities, without optional extensions or subsequent refinancing.
        // Held-for-sale loans are already in the gross rows; do not add their net reclassification again.
        let rows = crate::debt::rows(html, as_of, "Current Maturity")?;
        for row in rows.iter().filter(|row| {
            row.first().is_some_and(|label| {
                label.starts_with("Mortgage loan") || label.starts_with("Term Loan")
            })
        }) {
            anyhow::ensure!(
                matches!(
                    row.get(1).map(String::as_str),
                    Some(
                        "The Ritz-Carlton Lake Tahoe"
                            | "Capital Hilton"
                            | "Park Hyatt Beaver Creek Resort & Spa"
                            | "Land"
                            | "The Notary Hotel"
                            | "The Ritz-Carlton St. Thomas"
                            | "The Four Seasons Resort Scottsdale"
                    )
                ),
                "BHR loan repayment terms need review"
            );
        }
        let debt = crate::debt::bullets(
            html,
            as_of,
            "Current Maturity",
            (
                r"^(Mortgage loan|Term Loan|Convertible Senior Notes)(?: \(\d+\))*$",
                5,
                2,
            ),
            (
                "Indebtedness, net",
                &["Deferred loan costs, net", "Premiums/(discounts), net"],
            ),
        )?;
        anyhow::ensure!(
            i64::try_from(debt.total_debt_principal)?
                == crate::debt::scalar(html, "us-gaap:DebtInstrumentCarryingAmount", as_of)?
                    .context("missing BHR gross debt total")?,
            "BHR principal does not reconcile"
        );
        return Ok(debt);
    }
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
    for matched in Regex::new(r"(?i)TOTAL ENTERPRISE VALUE (?<date>[A-Za-z]+ \d{1,2}, 20\d{2}) \(in thousands, except share price\) \(unaudited\) [A-Za-z]+ \d{1,2}, 20\d{2} (?:Common stock shares outstanding|End of quarter common shares outstanding) [\d,]+ Partnership units outstanding \(common stock equivalents\) (?<count>[\d,]+)")?.captures_iter(text) {
                if let Some(end) = date(&matched["date"]) {
                    result.entry(end).or_default().outside_op_unit_share_equivalents = Some(Count {
                        value: matched["count"].replace(',', "").parse::<f64>()? * 1000.0,
                        source: source.into(), evidence: matched[0].into(), filed,
                    });
                }
            }

    Ok(())
}
