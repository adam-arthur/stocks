use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "GOOD",
        1234006,
        &[],
        vec![
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^ffo available to common stockholders(?: and non-controlling op unitholders)?\s*[-–—]\s*diluted$",
                1,
            ),
            (
                Metric::Ffo,
                Unit::Total,
                r"^funds from operations available to common stockholders(?: and non-controlling op unitholders)?\s*[-–—]\s*diluted$",
                1000,
            ),
        ],
        Some("ffo available to common stockholders"),
    ))?;
    company.tables = crate::table::joined_parentheses;
    company.debt = Some(debt);
    Ok(company)
}

fn debt(html: &scraper::Html, as_of: chrono::NaiveDate) -> anyhow::Result<crate::debt::Debt> {
    if chrono::Datelike::month(&as_of) != 12 {
        use anyhow::Context;
        let rows = crate::debt::rows(
            html,
            as_of,
            "Total mortgage notes payable, credit facility, and senior unsecured notes",
        )?;
        let debt = crate::debt::add_payments(
            crate::debt::tables(
                html,
                as_of,
                &[crate::debt::Rule {
                    intro: "Scheduled principal payments of mortgage notes payable",
                    ..crate::debt::Rule::new(0, 1000)
                }],
                None,
            )?,
            &crate::debt::instrument_payments(
                &rows,
                (
                    r"^(Variable rate (revolving credit facility|term loan facility [ABC])|Senior unsecured notes 20\d{2})$",
                    2,
                    5,
                ),
            )?,
        )?;
        let adjustment = |label: &str| -> anyhow::Result<i64> {
            let values = rows
                .iter()
                .filter(|row| row.first().is_some_and(|cell| cell == label))
                .filter_map(|row| row.get(2))
                .filter_map(|cell| crate::debt::amount(cell.trim_matches(['(', ')', ' ']), 1000))
                .collect::<Vec<_>>();
            anyhow::ensure!(values.len() == 1, "missing GOOD debt adjustment");
            Ok(i64::try_from(values[0])?)
        };
        anyhow::ensure!(
            i64::try_from(debt.total_debt_principal)?
                == crate::debt::scalar(html, "us-gaap:LongTermDebt", as_of)?
                    .context("missing GOOD debt balance")?
                    + adjustment("Deferred financing costs, mortgage loans, net")?
                    + adjustment("Deferred financing costs, term loan facility")?
                    + adjustment("Deferred financing costs, senior unsecured notes")?
                    - adjustment("Premiums and discounts, net")?,
            "GOOD principal does not reconcile"
        );
        return Ok(debt);
    }
    crate::debt::tables(
        html,
        as_of,
        &[crate::debt::Rule {
            anchor: "",
            heading: "Variable rate",
            intro: "future principal debt payment table",
            row: Some(("$", 0)),
            ..crate::debt::Rule::new(0, 1000)
        }],
        None,
    )
}
