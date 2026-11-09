use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "BRT",
        14846,
        &["brtapartments.com", "www.brtapartments.com"],
        vec![
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^(?:nareit )?funds from operations per (?:diluted common share|common share basic and diluted|common share - diluted)(?: - note 2)?$",
                1,
            ),
            (
                Metric::Ffo,
                Unit::Total,
                r"^(?:nareit )?funds from operations(?: attributable to common (?:stockholders|shareholders))?$",
                1000,
            ),
        ],
        None,
    ))?;
    company.debt = Some(debt);
    company.ownership = Some(crate::ownership::Rules::default());
    Ok(company)
}

fn debt(html: &scraper::Html, as_of: chrono::NaiveDate) -> anyhow::Result<crate::debt::Debt> {
    use anyhow::Context;
    use chrono::Datelike;
    // Require mortgage amortization by year; quarterly balances and refinance proceeds cannot update the annual schedule alone.
    let mut debt = crate::debt::tables(html, as_of, &[crate::debt::Rule::new(0, 1000)], None)
        .context(
            "BRT needs a complete mortgage repayment schedule, including scheduled amortization",
        )?;
    let text = crate::table::normalize(&html.root_element().text().collect::<Vec<_>>().join(" "));
    anyhow::ensure!(
        text.contains("The notes require interest only payments through the maturity date"),
        "BRT subordinated repayment terms changed"
    );
    let year = regex::Regex::new(r"The notes mature [A-Za-z]+ \d{1,2}, (20\d{2})")?
        .captures(&text)
        .context("missing BRT subordinated maturity")?[1]
        .parse::<i32>()?;
    let mut balances = std::collections::BTreeMap::new();
    for row in html.select(&scraper::Selector::parse("tr").unwrap()) {
        let cells: Vec<_> = row
            .select(&scraper::Selector::parse("td, th").unwrap())
            .map(|cell| crate::table::normalize(&cell.text().collect::<String>()))
            .filter(|cell| !cell.is_empty() && cell != "$")
            .collect();
        if cells.first().is_some_and(|cell| {
            matches!(
                cell.as_str(),
                "Mortgages payable"
                    | "Junior subordinated notes"
                    | "Credit facility"
                    | "Credit facility (1)"
            )
        }) {
            let amount =
                crate::debt::amount(cells.get(1).context("missing BRT debt balance")?, 1000)
                    .context("invalid BRT debt balance")?;
            if let Some(previous) = balances.insert(cells[0].clone(), amount) {
                anyhow::ensure!(previous == amount, "conflicting BRT debt balances");
            }
        }
    }
    anyhow::ensure!(
        balances.get("Mortgages payable") == Some(&debt.total_debt_principal),
        "BRT mortgage schedule does not reconcile"
    );
    anyhow::ensure!(
        balances
            .get("Credit facility")
            .or_else(|| balances.get("Credit facility (1)"))
            == Some(&0),
        "BRT drawn credit facility needs maturity allocation"
    );
    let principal = *balances
        .get("Junior subordinated notes")
        .context("missing BRT subordinated principal")?;
    anyhow::ensure!(year > as_of.year(), "BRT subordinated debt is overdue");
    if year > *debt.maturities.keys().next_back().unwrap() {
        debt.thereafter += principal;
    } else {
        *debt
            .maturities
            .get_mut(&year)
            .context("missing BRT maturity year")? += principal;
    }
    debt.total_debt_principal += principal;
    Ok(debt)
}
