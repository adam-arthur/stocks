use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "ELS",
        895417,
        &["equitylifestyle.gcs-web.com"],
        vec![(
            Metric::Ffo,
            Unit::PerShare,
            r#"^(?:funds from operations \([“"]ffo[”"]\)|ffo) per (?:fully diluted )?common share and op unit(?: - fully diluted)?$"#,
            1,
        )],
        None,
    ))?;
    company.tables = crate::table::joined_parentheses;
    company.official_url = |url| {
        url.host_str() == Some("www.prnewswire.com")
            && url.path().starts_with("/news-releases/els-reports-")
    };
    company.debt = Some(debt);
    Ok(company)
}

fn debt(html: &scraper::Html, as_of: chrono::NaiveDate) -> anyhow::Result<crate::debt::Debt> {
    use anyhow::Context;
    // The quarterly mortgage total and maturity range cannot allocate amortization or balloons to calendar years.
    crate::debt::tables(html, as_of, &[crate::debt::Rule::new(0, 1000)], None).context(
        "ELS needs a mortgage repayment schedule; aggregate maturity ranges are insufficient",
    )
}
