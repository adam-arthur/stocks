use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "CHCT",
        1631569,
        &["www.chct.reit", "investors.chct.reit"],
        vec![
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^(?:funds from operations|ffo) per common share(?:\s*-\s*diluted)?$",
                1,
            ),
            (Metric::Ffo, Unit::Total, r"^funds from operations$", 1000),
        ],
        None,
    ))?;
    company.tables = crate::table::joined_parentheses;
    company.debt = Some(debt);
    Ok(company)
}

fn debt(html: &scraper::Html, as_of: chrono::NaiveDate) -> anyhow::Result<crate::debt::Debt> {
    use anyhow::Context;
    let text = crate::table::normalize(&html.root_element().text().collect::<Vec<_>>().join(" "));
    anyhow::ensure!(text.contains("Loans under the Credit Facility are interest only with principal amounts due as of each facility's applicable maturity date"), "CHCT facility repayment terms changed");
    let mut payments = Vec::new();
    let mut carrying = 0;
    for (label, pattern) in [
        ("Revolving Credit Facility", None),
        (
            "A-4 Term Loan, net",
            Some(r#"aggregate principal amount of \$ ([\d.]+) million \(the "A-4 Term Loan"\)"#),
        ),
        (
            "A-5 Term Loan, net",
            Some(r#"aggregate principal amount of \$ ([\d.]+) million \(the "A-5 Term Loan"\)"#),
        ),
    ] {
        let row = html
            .select(&scraper::Selector::parse("tr").unwrap())
            .find_map(|row| {
                let cells: Vec<_> = row
                    .select(&scraper::Selector::parse("td, th").unwrap())
                    .map(|cell| crate::table::normalize(&cell.text().collect::<String>()))
                    .filter(|cell| !cell.is_empty() && cell != "$")
                    .collect();
                (cells.first().is_some_and(|cell| cell == label)
                    && cells.last().is_some_and(|cell| {
                        regex::Regex::new(r"^\d{1,2}/(?:\d{1,2}/)?20\d{2}$")
                            .unwrap()
                            .is_match(cell)
                    }))
                .then_some(cells)
            })
            .context("missing CHCT facility balance and maturity")?;
        carrying += row[1].replace(',', "").parse::<u64>()? * 1000;
        payments.push((
            row.last()
                .unwrap()
                .split('/')
                .next_back()
                .unwrap()
                .parse()?,
            match pattern {
                Some(pattern) => (regex::Regex::new(pattern)?
                    .captures(&text)
                    .context("missing CHCT term loan principal")?[1]
                    .parse::<f64>()?
                    * 1_000_000.0)
                    .round() as u64,
                None => row[1].replace(',', "").parse::<u64>()? * 1000,
            },
        ));
    }
    anyhow::ensure!(
        crate::debt::scalar(html, "us-gaap:LongTermDebt", as_of)? == Some(i64::try_from(carrying)?),
        "CHCT facilities do not reconcile to total debt"
    );
    crate::debt::payments(as_of, &payments)
}
