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
    company.supplements = Some(|sources, company, since, through| {
        Box::pin(async move {
            let mut releases = supplements(since, through);
            for (_, url, date) in &mut releases {
                let html = sources.get(company, url).await?;
                let document = scraper::Html::parse_document(&html);
                let selector = scraper::Selector::parse(".module_date-text").unwrap();
                let text = document
                    .select(&selector)
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("missing release date: {url}"))?
                    .text()
                    .collect::<String>();
                *date = Some(chrono::NaiveDate::parse_from_str(text.trim(), "%m/%d/%Y")?);
            }
            Ok(releases)
        })
    });
    company.supplement_period_required = true;
    company.debt = Some(debt);
    company.affo_support = crate::AffoSupport::Unavailable;
    Ok(company)
}

pub fn supplements(
    since: chrono::NaiveDate,
    through: chrono::NaiveDate,
) -> Vec<(chrono::NaiveDate, String, Option<chrono::NaiveDate>)> {
    use chrono::Datelike;

    (since.year()..=through.year())
        .map(|year| (
            chrono::NaiveDate::from_ymd_opt(year, 12, 31).unwrap(),
            format!("https://alx-inc.com/press-release/news-details/{}/Alexanders-Announces-Fourth-Quarter-{}Results/default.aspx", year + 1, if year <= 2017 { "" } else { "Financial-" }),
            None,
        ))
        .filter(|(end, _, _)| *end >= since && *end <= through)
        .collect()
}

fn debt(html: &scraper::Html, as_of: chrono::NaiveDate) -> anyhow::Result<crate::debt::Debt> {
    if chrono::Datelike::month(&as_of) != 12 {
        // The annual disclosures establish bullet repayments for these loans; use quarterly balances and dates.
        // Include accrued C-Note PIK, excluding intercompany A/B notes and future PIK.
        return crate::debt::bullets(
            html,
            as_of,
            "First mortgages secured by:",
            (
                r"^(731 Lexington Avenue, (office|retail) condominium(?:\s*\(\d+\))?|Rego Park (?:II )?shopping center(?:\s*\(\d+\))+|The Alexander apartment tower)$",
                3,
                1,
            ),
            ("Total", &[]),
        );
    }
    crate::debt::tables(html, as_of, &[crate::debt::Rule::new(0, 1000)], None)
}
