use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "ALX",
        3499,
        &["www.alx-inc.com", "alx-inc.com"],
        vec![
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^ffo per diluted share(?: \(non-gaap\))?$",
                1,
            ),
            (Metric::Ffo, Unit::Total, r"^ffo(?: \(non-gaap\))?$", 1000),
        ],
        None,
    ))?;
    company.filings = &["10-Q", "10-Q/A", "10-K", "10-K/A"];
    company.supplements =
        Some(|_, _, since, through| Box::pin(async move { Ok(supplements(since, through)) }));
    company.supplement_period_required = true;
    Ok(company)
}

pub fn supplements(
    since: chrono::NaiveDate,
    through: chrono::NaiveDate,
) -> Vec<(chrono::NaiveDate, String)> {
    use chrono::Datelike;

    (since.year()..=through.year())
        .map(|year| (
            chrono::NaiveDate::from_ymd_opt(year, 12, 31).unwrap(),
            format!("https://alx-inc.com/press-release/news-details/{}/Alexanders-Announces-Fourth-Quarter-Financial-Results/default.aspx", year + 1),
        ))
        .filter(|(end, _)| *end >= since && *end <= through)
        .collect()
}
