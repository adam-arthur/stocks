use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "FVR",
        1988494,
        &[],
        vec![
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^ffo per share(?:, diluted)?$",
                1,
            ),
            (
                Metric::Ffo,
                Unit::Total,
                r"^(?:funds from operations|ffo)$",
                1000,
            ),
        ],
        None,
    ))?;
    company.tables = tables;
    company.debt = Some(debt);
    company.ownership = Some(crate::ownership::Rules {
        units: &[(
            "us-gaap:CommonUnitOutstanding",
            &["fvr:OwnershipInterestMember"],
        )],
        exchange_pattern: Some(
            r"(?i)OP Units.{0,160}redeemable into the Company.s common stock.{0,100}one\s*-for-one basis",
        ),
        ..Default::default()
    });
    Ok(company)
}

fn tables(html: &str) -> Vec<crate::table::Table> {
    crate::table::joined_parentheses(html)
        .into_iter()
        .map(|mut table| {
            let pro_forma = table
                .rows
                .iter()
                .take(8)
                .flatten()
                .filter(|cell| cell.text.to_lowercase().contains("pro forma"))
                .cloned()
                .collect::<Vec<_>>();
            for cell in table.rows.iter_mut().flatten() {
                if pro_forma.iter().any(|header| header.covers(cell.column))
                    && (crate::number(&cell.text).is_some()
                        || cell.text.to_lowercase().contains("pro forma"))
                {
                    cell.text.clear();
                }
            }
            table
        })
        .collect()
}

fn debt(html: &scraper::Html, as_of: chrono::NaiveDate) -> anyhow::Result<crate::debt::Debt> {
    use anyhow::Context;
    let principal = crate::debt::scalar(html, "us-gaap:LongTermDebt", as_of)?
        .context("missing total debt")?
        - crate::debt::scalar(
            html,
            "fvr:DebtInstrumentUnamortizedFinancingTransactionCosts",
            as_of,
        )?
        .context("missing financing costs")?;
    let table = html
        .select(&scraper::Selector::parse("table").unwrap())
        .find(|table| {
            table
                .select(&scraper::Selector::parse("[name]").unwrap())
                .any(|node| {
                    node.value().attr("name").is_some_and(|name| {
                        name.starts_with("us-gaap:LongTermDebtMaturitiesRepaymentsOfPrincipal")
                    })
                })
        })
        .context("missing FVR debt schedule")?;
    let mut payments = Vec::new();
    for row in table.select(&scraper::Selector::parse("tr").unwrap()) {
        let cells: Vec<_> = row
            .select(&scraper::Selector::parse("td, th").unwrap())
            .map(|cell| crate::table::normalize(&cell.text().collect::<String>()))
            .filter(|cell| !cell.is_empty() && cell != "$")
            .collect();
        if let Some(year) = cells.first().and_then(|cell| cell.parse::<i32>().ok()) {
            payments.push((
                year,
                crate::debt::amount(cells.get(1).context("missing FVR principal")?, 1000)
                    .context("invalid FVR principal")?,
            ));
        }
    }
    let debt = crate::debt::payments(as_of, &payments)?;
    anyhow::ensure!(
        i64::try_from(debt.total_debt_principal)? == principal,
        "FVR schedule does not reconcile to principal"
    );
    Ok(debt)
}
