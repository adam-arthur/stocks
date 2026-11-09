use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "CURB",
        2027317,
        &[],
        vec![
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^ffo per share\s*[-–—]\s*diluted$",
                1,
            ),
            (
                Metric::Ffo,
                Unit::Total,
                r"^ffo attributable to curbline$",
                1000,
            ),
        ],
        None,
    ))?;
    company.tables = tables;
    company.debt = Some(debt);
    Ok(company)
}

fn tables(html: &str) -> Vec<crate::table::Table> {
    let period = regex::Regex::new(r"^([1-4])Q(\d{2})$|^(3|6|9|12)M(\d{2})$|^FY(\d{2})$").unwrap();
    crate::table::joined_parentheses(html)
        .into_iter()
        .map(|mut table| {
            for cell in table.rows.iter_mut().flatten() {
                if let Some(period) = period.captures(&cell.text) {
                    cell.text = if let Some(quarter) = period.get(1) {
                        format!("Q{} 20{}", quarter.as_str(), &period[2])
                    } else if let Some(months) = period.get(3) {
                        format!(
                            "{} months ended {} 20{}",
                            match months.as_str() {
                                "3" => "Three",
                                "6" => "Six",
                                "9" => "Nine",
                                _ => "Twelve",
                            },
                            match months.as_str() {
                                "3" => "March 31",
                                "6" => "June 30",
                                "9" => "September 30",
                                _ => "December 31",
                            },
                            &period[4]
                        )
                    } else {
                        format!("Year ended December 31 20{}", &period[5])
                    };
                }
            }
            table
        })
        .collect()
}

fn debt(html: &scraper::Html, as_of: chrono::NaiveDate) -> anyhow::Result<crate::debt::Debt> {
    use anyhow::Context;
    if chrono::Datelike::month(&as_of) != 12 {
        let rows = crate::debt::rows(html, as_of, "Total indebtedness")?;
        let text =
            crate::table::normalize(&html.root_element().text().collect::<Vec<_>>().join(" "));
        anyhow::ensure!(
            crate::debt::scalar(html, "us-gaap:LineOfCredit", as_of)? == Some(0),
            "CURB drawn revolver needs a maturity date"
        );
        // Split note groups using disclosed tranche terms only when issued principal reconciles to current outstanding balances.
        // Use loan maturities rather than swap expirations or unexercised extensions; exclude planned undrawn loans.
        let mut notes = std::collections::BTreeMap::new();
        for matched in regex::Regex::new(
            r#"\$\s*([\d.]+) million aggregate principal amount of [\d.]+\s*% senior unsecured notes due [A-Za-z]+ \d{1,2}, (20\d{2}) \(the [“"](20\d{2}-[A-Z]) Notes[”"]\)"#
        )?.captures_iter(&text) {
            let payment = (matched[2].parse::<i32>()?, crate::debt::amount(&matched[1], 1_000_000).context("invalid CURB tranche amount")?);
            if let Some(previous) = notes.insert(matched[3].to_string(), payment) {
                anyhow::ensure!(previous == payment, "conflicting CURB tranche terms");
            }
        }
        let mut payments = crate::debt::instrument_payments(&rows, (r"^20\d{2} Term Loan$", 1, 4))?;
        for (group, tranches) in [
            ("2025 Notes", ["2025-A", "2025-B"]),
            ("2026 Notes", ["2025-C", "2026-A"]),
        ] {
            let part = tranches
                .iter()
                .map(|name| {
                    notes
                        .get(*name)
                        .copied()
                        .context("missing CURB note tranche")
                })
                .collect::<Result<Vec<_>>>()?;
            anyhow::ensure!(
                Some(part.iter().map(|(_, amount)| amount).sum::<u64>())
                    == rows
                        .iter()
                        .find(|row| row.first().is_some_and(|label| label == group))
                        .and_then(|row| row.get(1))
                        .and_then(|value| crate::debt::amount(value, 1000)),
                "CURB tranche issuance differs from outstanding {group}; repayments need allocation"
            );
            payments.extend(part);
        }
        let debt = crate::debt::payments(as_of, &payments)?;
        let net = rows
            .iter()
            .find(|row| {
                row.first()
                    .is_some_and(|label| label == "Total indebtedness")
            })
            .and_then(|row| row.get(1))
            .and_then(|value| crate::debt::amount(value, 1000))
            .context("missing CURB net debt")?;
        let costs = rows
            .iter()
            .filter(|row| {
                row.first()
                    .is_some_and(|label| label == "Net unamortized debt issuance costs")
            })
            .map(|row| {
                row.get(1)
                    .and_then(|value| {
                        crate::debt::amount(value.trim_matches(['(', ')', ' ']), 1000)
                    })
                    .context("invalid CURB debt issuance costs")
            })
            .collect::<Result<Vec<_>>>()?;
        anyhow::ensure!(
            costs.len() == 2 && debt.total_debt_principal == net + costs.iter().sum::<u64>(),
            "CURB maturity principal does not reconcile"
        );
        return Ok(debt);
    }
    crate::debt::tables(html, as_of, &[crate::debt::Rule::new(0, 1000)], None)
}
