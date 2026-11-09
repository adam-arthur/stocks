use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "ESS",
        920522,
        &["investors.essexapartmenthomes.com"],
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
                r"^(?:ffo|funds from operations) attributable to common stockholders and unitholders$",
                1000,
            ),
        ],
        None,
    ))?;
    company.tables = crate::table::joined_parentheses;
    company.debt = Some(debt);
    Ok(company)
}

fn debt(html: &scraper::Html, as_of: chrono::NaiveDate) -> anyhow::Result<crate::debt::Debt> {
    if chrono::Datelike::month(&as_of) != 12 {
        // The quarterly schedule excludes credit lines and commercial paper; accept it only when both current balances are zero.
        let rows = crate::debt::rows(html, as_of, "Total debt, net")?;
        for prefix in ["Lines of credit", "Commercial paper"] {
            let balances: Vec<_> = rows
                .iter()
                .filter(|row| row.first().is_some_and(|label| label.starts_with(prefix)))
                .map(|row| {
                    row.get(1)
                        .and_then(|value| crate::debt::amount(value, 1000))
                })
                .collect();
            anyhow::ensure!(
                balances == [Some(0)],
                "ESS {prefix} needs separate maturity allocation"
            );
        }
        let debt = crate::debt::tables(
            html,
            as_of,
            &[crate::debt::Rule {
                intro: "excluding lines of credit and commercial paper",
                ..crate::debt::Rule::new(0, 1000)
            }],
            None,
        )?;
        anyhow::ensure!(
            Some(i64::try_from(debt.total_debt_principal)?)
                == crate::debt::scalar(html, "us-gaap:LongTermDebt", as_of)?,
            "ESS principal does not reconcile to the reported schedule total"
        );
        return Ok(debt);
    }
    crate::debt::tables(
        html,
        as_of,
        &(["Fixed rate debt", "Variable rate debt"]
            .into_iter()
            .map(|row| crate::debt::Rule {
                anchor: "",
                heading: "Fair value",
                intro: "scheduled principal payments",
                row: Some((row, 0)),
                ..crate::debt::Rule::new(0, 1000)
            })
            .collect::<Vec<_>>()),
        None,
    )
}
