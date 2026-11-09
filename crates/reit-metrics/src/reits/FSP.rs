use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "FSP",
        1031316,
        &[],
        vec![(Metric::Ffo, Unit::Total, r"^nareit ffo$", 1000)],
        None,
    ))?;
    company.tables = crate::table::joined_parentheses;
    company.debt = Some(debt);
    company.ownership = Some(crate::ownership::Rules::default());
    Ok(company)
}

fn debt(html: &scraper::Html, as_of: chrono::NaiveDate) -> anyhow::Result<crate::debt::Debt> {
    use anyhow::Context;
    let text = crate::table::normalize(&html.root_element().text().collect::<Vec<_>>().join(" "));
    let years: std::collections::BTreeSet<_> = regex::Regex::new(r"The Term Loans are not subject to amortization and have an initial stated maturity date of [A-Za-z]+ \d{1,2}, (20\d{2})")?.captures_iter(&text).map(|matched| matched[1].parse::<i32>()).collect::<Result<_, _>>()?;
    anyhow::ensure!(years.len() == 1, "missing FSP bullet loan maturity");
    let cost_pattern = regex::Regex::new(
        r"^Initial Term Loans, less unamortized financing costs and OID of \$([\d,]+)$",
    )?;
    let mut principal = None;
    let mut retired = 0;
    for row in html.select(&scraper::Selector::parse("tr").unwrap()) {
        let cells: Vec<_> = row
            .select(&scraper::Selector::parse("td, th").unwrap())
            .map(|cell| crate::table::normalize(&cell.text().collect::<String>()))
            .filter(|cell| !cell.is_empty() && cell != "$")
            .collect();
        let Some(label) = cells.first() else { continue };
        if let Some(costs) = cost_pattern.captures(label) {
            let carrying =
                crate::debt::amount(cells.get(1).context("missing FSP loan balance")?, 1000)
                    .context("invalid FSP loan balance")?;
            anyhow::ensure!(
                crate::debt::scalar(html, "us-gaap:SecuredDebt", as_of)?
                    == Some(i64::try_from(carrying)?),
                "FSP debt balance does not match current period"
            );
            principal = Some(
                carrying
                    + crate::debt::amount(&costs[1], 1000)
                        .context("invalid FSP financing costs")?,
            );
        }
        if label.starts_with("Term loans payable, less")
            || label.starts_with("Series A & Series B Senior Notes, less")
        {
            anyhow::ensure!(
                cells
                    .get(1)
                    .and_then(|cell| crate::debt::amount(cell, 1000))
                    == Some(0),
                "FSP retains other outstanding debt"
            );
            retired += 1;
        }
    }
    anyhow::ensure!(retired == 2, "missing FSP retired debt balances");
    crate::debt::payments(
        as_of,
        &[(
            *years.first().unwrap(),
            principal.context("missing FSP principal")?,
        )],
    )
}
