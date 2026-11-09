use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "AIV",
        922864,
        &["www.aimco.com", "investors.aimco.com"],
        vec![
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^(?:nareit )?ffo per share\s*[-–—]\s*diluted$",
                1,
            ),
            (
                Metric::Ffo,
                Unit::Total,
                r"^(?:nareit )?ffo attributable to aimco common stockholders$",
                1000,
            ),
        ],
        None,
    ))?;
    company.debt = Some(debt);
    Ok(company)
}

fn debt(html: &scraper::Html, as_of: chrono::NaiveDate) -> anyhow::Result<crate::debt::Debt> {
    use anyhow::Context;
    // Liquidation settlement estimates and planned sales do not establish contractual principal maturity years.
    crate::debt::tables(
        html,
        as_of,
        &([
            "non-recourse property debt were as follows",
            "non-recourse construction loans were as follows",
        ]
        .into_iter()
        .map(|intro| crate::debt::Rule {
            anchor: "us-gaap:LongTermDebtMaturitiesRepaymentsOfPrincipalAfterYearFive",
            intro,
            ..crate::debt::Rule::new(0, 1000)
        })
        .collect::<Vec<_>>()),
        None,
    )
    .context("AIV needs dated principal payments; quarterly liquidation balances do not provide a maturity schedule")
}
