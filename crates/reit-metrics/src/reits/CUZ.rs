use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "CUZ",
        25232,
        &[],
        vec![
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^ffo per share(?: - basic and diluted)?$",
                1,
            ),
            (Metric::Ffo, Unit::Total, r"^ffo$", 1000),
        ],
        None,
    ))?;
    company.tables = tables;
    company.debt = Some(debt);
    Ok(company)
}

fn tables(html: &str) -> Vec<crate::table::Table> {
    let quarter = regex::Regex::new(r"^(20\d{2}) ([1-4])(?:st|nd|rd|th)$").unwrap();
    crate::table::joined_parentheses(html)
        .into_iter()
        .map(|mut table| {
            for cell in table.rows.iter_mut().take(8).flatten() {
                if let Some(period) = quarter.captures(&cell.text) {
                    cell.text = format!("Q{} {}", &period[2], &period[1]);
                }
            }
            table
        })
        .collect()
}

fn debt(html: &scraper::Html, as_of: chrono::NaiveDate) -> anyhow::Result<crate::debt::Debt> {
    use anyhow::Context;
    use chrono::Datelike;
    if as_of.month() != 12 {
        let mut rows = crate::debt::rows(html, as_of, "Total Notes Payable")?;
        // Terminus and unsecured borrowings repay at maturity; amortizing mortgages must mature within the current year.
        // Keep quarter-end principal even when a subsequent-event repayment is disclosed; exclude unconsolidated JV debt.
        for row in &mut rows {
            if row
                .first()
                .is_some_and(|label| matches!(label.as_str(), "201 North Tryon" | "Colorado Tower"))
            {
                anyhow::ensure!(
                    row.get(3)
                        .and_then(|value| crate::debt::amount(value, 1000))
                        == Some(0)
                        || row
                            .get(2)
                            .is_some_and(|date| date.ends_with(&as_of.year().to_string())),
                    "CUZ amortizing mortgage needs annual principal payments"
                );
            }
            if row
                .first()
                .is_some_and(|label| label == "Privately Placed Senior Notes")
            {
                row[0] = format!(
                    "{} {}",
                    row[0],
                    row.get(1).context("missing CUZ note coupon")?
                );
            }
        }
        let debt = crate::debt::payments(
            as_of,
            &crate::debt::instrument_payments(
                &rows,
                (
                    r"^(Credit Facility$|Public Senior Notes$|Privately Placed Senior Notes |Term Loan(?: \(\d+\))?$|Terminus(?: \(\d+\))?$|201 North Tryon$|Colorado Tower$)",
                    3,
                    2,
                ),
            )?,
        )?;
        anyhow::ensure!(
            Some(i64::try_from(debt.total_debt_principal)?)
                == crate::debt::scalar(html, "us-gaap:DebtInstrumentCarryingAmount", as_of)?,
            "CUZ maturity principal does not reconcile"
        );
        return Ok(debt);
    }
    crate::debt::tables(html, as_of, &[crate::debt::Rule::new(0, 1000)], None)
}
