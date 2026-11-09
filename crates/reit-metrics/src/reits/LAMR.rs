use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "LAMR",
        1090425,
        &[],
        vec![(Metric::Ffo, Unit::Total, r"^funds from operations$", 1000)],
        None,
    ))?;
    company.tables = crate::table::joined_parentheses;
    company.official_url = |url| {
        url.host_str() == Some("www.sec.gov")
            && url.path().starts_with("/Archives/edgar/data/899045/")
    };
    company.debt = Some(debt);
    company.ownership = Some(crate::ownership::Rules {
        classes: &[
            (
                "us-gaap:commonstocksharesoutstanding",
                &["us-gaap:commonclassamember"],
            ),
            (
                "us-gaap:commonstocksharesissued",
                &["us-gaap:commonclassbmember"],
            ),
        ],
        ..Default::default()
    });
    Ok(company)
}

fn debt(html: &scraper::Html, as_of: chrono::NaiveDate) -> anyhow::Result<crate::debt::Debt> {
    crate::debt::tables(html, as_of, &[crate::debt::Rule::new(0, 1000)], None)
}
