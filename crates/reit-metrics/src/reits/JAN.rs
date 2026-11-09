use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "JAN",
        2100805,
        &["www.janusreit.com", "ir.janusreit.com"],
        vec![
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^diluted nareit ffo per common share$",
                1,
            ),
            (Metric::Ffo, Unit::Total, r"^diluted nareit ffo$", 1000),
        ],
        None,
    ))?;
    company.debt = Some(debt);
    Ok(company)
}

fn debt(html: &scraper::Html, as_of: chrono::NaiveDate) -> anyhow::Result<crate::debt::Debt> {
    let text = crate::table::normalize(&html.root_element().text().collect::<Vec<_>>().join(" "));
    anyhow::ensure!(
        crate::debt::scalar(html, "us-gaap:SecuredDebt", as_of)? == Some(0)
            && text.contains("reducing our outstanding consolidated debt to zero")
            && text.contains(&format!(
                "No principal had been drawn on our Credit Facilities as of {}",
                as_of.format("%B %-d, %Y")
            )),
        "JAN requires a complete nonzero debt schedule"
    );
    crate::debt::payments(as_of, &[])
}
