use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "NLCP",
        1854964,
        &["ir.newlake.com", "www.newlake.com"],
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
                r"^ffo attributable to common stockholders\s*[-–—]\s*diluted$",
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
    let mut liabilities = 0;
    for tag in [
        "AccountsPayableAndAccruedLiabilitiesCurrentAndNoncurrent",
        "DividendsPayableCurrentAndNoncurrent",
        "SecurityDepositLiability",
        "AdvanceRent",
        "OtherLiabilities",
    ] {
        liabilities += crate::debt::scalar(html, &format!("us-gaap:{tag}"), as_of)?
            .context("missing NLCP liability")?;
    }
    anyhow::ensure!(
        crate::debt::scalar(html, "us-gaap:LineOfCredit", as_of)? == Some(0)
            && crate::debt::scalar(html, "us-gaap:Liabilities", as_of)? == Some(liabilities),
        "NLCP requires a nonzero borrowing schedule"
    );
    crate::debt::payments(as_of, &[])
}
