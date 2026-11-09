use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "AHT",
        1232582,
        &["www.ahtreit.com", "ahtreit.com"],
        vec![
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^ffo per diluted share(?: available to common stockholders and op unitholders)?$",
                1,
            ),
            (
                Metric::Ffo,
                Unit::Total,
                r"^ffo available to common stockholders and op unitholders$",
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
    // Require scheduled amortization as well as balloons; quarterly loan balances include overdue debt and an amortizing term loan.
    crate::debt::tables(html, as_of, &[crate::debt::Rule::new(0, 1000)], None).context(
        "AHT needs a complete repayment schedule; loan maturity dates alone omit amortization",
    )
}
