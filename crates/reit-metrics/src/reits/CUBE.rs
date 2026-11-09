use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "CUBE",
        1298675,
        &[],
        vec![
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^ffo per (?:share and unit - fully diluted|diluted share and unit)$",
                1,
            ),
            (
                Metric::Ffo,
                Unit::Total,
                r"^ffo attributable to (?:the company's )?common shareholders and (?:third-party )?op unitholders$",
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
    use anyhow::Context;
    use chrono::Datelike;
    let mortgage = crate::debt::tables(
        html,
        as_of,
        &[crate::debt::Rule {
            anchor: "",
            intro: "future principal payment requirements on the outstanding mortgage loans and notes payable",
            ..crate::debt::Rule::new(0, 1000)
        }],
        None,
    )?;
    let mut payments: Vec<_> = mortgage.maturities.into_iter().collect();
    payments.push((as_of.year() + 6, mortgage.thereafter));
    let bond_year = regex::Regex::new(r"Guaranteed Notes due (20\d{2})")?;
    let mut bonds = std::collections::BTreeMap::new();
    for table in html
        .select(&scraper::Selector::parse("table").unwrap())
        .filter(|table| {
            let text = crate::table::normalize(&table.text().collect::<Vec<_>>().join(" "));
            text.contains("Unsecured Senior Notes")
                && text.contains("Principal balance outstanding")
        })
    {
        for row in table.select(&scraper::Selector::parse("tr").unwrap()) {
            let cells: Vec<_> = row
                .select(&scraper::Selector::parse("td, th").unwrap())
                .map(|cell| crate::table::normalize(&cell.text().collect::<String>()))
                .filter(|cell| !cell.is_empty() && cell != "$")
                .collect();
            if let Some(label) = cells.first()
                && let Some(year) = bond_year.captures(label)
            {
                let amount =
                    crate::debt::amount(cells.get(1).context("missing CUBE bond principal")?, 1000)
                        .context("invalid CUBE bond principal")?;
                if let Some(previous) =
                    bonds.insert(label.clone(), (year[1].parse::<i32>()?, amount))
                {
                    anyhow::ensure!(previous.1 == amount, "conflicting CUBE bond balances");
                }
            }
        }
    }
    anyhow::ensure!(
        Some(i64::try_from(
            bonds.values().map(|(_, amount)| amount).sum::<u64>()
        )?) == crate::debt::scalar(html, "us-gaap:DebtInstrumentFaceAmount", as_of)?,
        "CUBE bond principal does not reconcile"
    );
    payments.extend(bonds.into_values().filter(|(_, amount)| *amount > 0));
    let text = crate::table::normalize(&html.root_element().text().collect::<Vec<_>>().join(" "));
    let years: std::collections::BTreeSet<_> = regex::Regex::new(r#"revolving credit facility \(the [“"]Revolver[”"]\) maturing on [A-Za-z]+ \d{1,2}, (20\d{2})"#)?.captures_iter(&text).map(|matched| matched[1].parse::<i32>()).collect::<Result<_, _>>()?;
    anyhow::ensure!(years.len() == 1, "ambiguous CUBE revolver maturity");
    payments.push((
        *years.first().unwrap(),
        u64::try_from(
            crate::debt::scalar(html, "us-gaap:LineOfCredit", as_of)?
                .context("missing CUBE revolver balance")?,
        )?,
    ));
    crate::debt::payments(as_of, &payments)
}
