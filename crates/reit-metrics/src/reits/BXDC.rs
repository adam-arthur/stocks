use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "BXDC",
        2100161,
        &[],
        vec![
            (Metric::Ffo, Unit::Total, r"^ffo$", 1000),
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^ffo per adjusted share, basic and diluted$",
                1,
            ),
            (Metric::Affo, Unit::Total, r"^affo$", 1000),
            (
                Metric::Affo,
                Unit::PerShare,
                r"^affo per adjusted share, basic and diluted$",
                1,
            ),
        ],
        None,
    ))?;
    company.affo_support = crate::AffoSupport::Reported;
    company.debt = Some(|html, as_of| {
        let rows = crate::debt::rows(html, as_of, "Total Liabilities and Equity")?;
        let amount = |label| {
            rows.iter()
                .find(|row| row.first().is_some_and(|s| s == label))
                .and_then(|row| row.get(1))
                .and_then(|value| crate::debt::amount(value, 1000))
                .ok_or_else(|| anyhow::anyhow!("missing balance-sheet line {label}"))
        };
        let text =
            crate::table::normalize(&html.root_element().text().collect::<Vec<_>>().join(" "));
        let date = as_of.format("%B %-d, %Y");
        anyhow::ensure!(
            text.contains(&format!(
                "As of {date}, we had no outstanding borrowings under the Credit Facility"
            )) && amount("Total Liabilities")?
                == amount("Due to affiliates")? + amount("Other liabilities")?,
            "BXDC balance sheet does not confirm zero borrowings"
        );
        crate::debt::payments(as_of, &[])
    });
    Ok(company)
}
