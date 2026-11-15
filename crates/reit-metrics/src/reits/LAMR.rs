use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "LAMR",
        1090425,
        &[],
        vec![
            (Metric::Affo, Unit::PerShare, r"^diluted affo per share$", 1),
            (
                Metric::Affo,
                Unit::Total,
                r"^adjusted funds from operations$",
                1000,
            ),
            (Metric::Ffo, Unit::Total, r"^funds from operations$", 1000),
        ],
        None,
    ))?;
    company.affo_support = crate::AffoSupport::Reported;
    company.tables = |html| {
        crate::table::joined_parentheses(html)
            .into_iter()
            .filter(|table| {
                !table.rows.iter().take(6).any(|row| {
                    row.iter().any(|cell| cell.text.eq_ignore_ascii_case("low"))
                        && row
                            .iter()
                            .any(|cell| cell.text.eq_ignore_ascii_case("high"))
                })
            })
            .collect()
    };
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
