use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "COLD",
        1455863,
        &["www.americold.com", "ir.americold.com"],
        vec![(
            Metric::Ffo,
            Unit::PerShare,
            r"^nareit ffo(?: available to common shareholders)? - diluted per share$",
            1,
        )],
        None,
    ))?;
    company.tables = tables;
    company.debt = Some(debt);
    Ok(company)
}

fn tables(html: &str) -> Vec<crate::table::Table> {
    let mut tables = crate::table::joined_parentheses(html);
    let quarter = regex::Regex::new(r"^Q([1-4]) (\d{2})$").unwrap();
    for cell in tables
        .iter_mut()
        .flat_map(|table| table.rows.iter_mut().flatten())
    {
        if let Some(period) = quarter.captures(&cell.text) {
            cell.text = format!("Q{} 20{}", &period[1], &period[2]);
        }
    }
    tables
}

fn debt(html: &scraper::Html, as_of: chrono::NaiveDate) -> anyhow::Result<crate::debt::Debt> {
    use anyhow::Context;
    if chrono::Datelike::month(&as_of) != 12 {
        // Use instrument dates rather than relabeling rolling twelve-month buckets as calendar years.
        // Use reported USD balances, exclude lease financing, and do not assume revolver extensions.
        let mut payments = Vec::new();
        for (heading, pattern) in [
            (
                "Total Senior Unsecured Notes",
                r"^(Private Series [A-Z] Notes|Public [\d.]+% Notes)",
            ),
            (
                "Total Senior Unsecured Term Loans",
                r"^(20\d{2} Unsecured Term Loan|AUD Term Loan Facility|Tranche A-\d|Delayed Draw Tranche A-\d)",
            ),
        ] {
            let rows: Vec<Vec<String>> = crate::debt::rows(html, as_of, heading)?
                .into_iter()
                .map(|row| {
                    row.into_iter()
                        .filter(|cell| !matches!(cell.as_str(), "€" | "A$" | "C$"))
                        .collect()
                })
                .collect();
            let part = crate::debt::instrument_payments(&rows, (pattern, 4, 1))?;
            anyhow::ensure!(
                Some(part.iter().map(|(_, amount)| amount).sum::<u64>())
                    == rows
                        .iter()
                        .find(|row| row.first().is_some_and(|label| label == heading))
                        .and_then(|row| row.get(1))
                        .and_then(|value| crate::debt::amount(value, 1000)),
                "COLD instrument balances do not reconcile to {heading}"
            );
            payments.extend(part);
        }
        let text =
            crate::table::normalize(&html.root_element().text().collect::<Vec<_>>().join(" "));
        let years: std::collections::BTreeSet<_> = regex::Regex::new(
            r"Senior Unsecured Revolving Credit Facility is comprised of .{0,600}?maturity date was extended to [A-Za-z]+ (20\d{2})"
        )?.captures_iter(&text).map(|matched| matched[1].parse::<i32>()).collect::<Result<_, _>>()?;
        anyhow::ensure!(
            years.len() == 1,
            "missing or conflicting COLD revolver maturity"
        );
        payments.push((
            *years.first().unwrap(),
            u64::try_from(
                crate::debt::scalar(html, "us-gaap:LineOfCredit", as_of)?
                    .context("missing COLD revolver balance")?,
            )?,
        ));
        let debt = crate::debt::payments(as_of, &payments)?;
        anyhow::ensure!(
            Some(i64::try_from(debt.total_debt_principal)?)
                == crate::debt::scalar(html, "us-gaap:DebtInstrumentCarryingAmount", as_of)?,
            "COLD maturity principal does not reconcile"
        );
        return Ok(debt);
    }
    crate::debt::tables(html, as_of, &[crate::debt::Rule::new(0, 1000)], None)
}
