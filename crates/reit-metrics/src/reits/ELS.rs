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
    Ok(company)
}
