use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "ELS",
        895417,
        &["equitylifestyle.gcs-web.com"],
        vec![
            (
                Metric::Affo,
                Unit::PerShare,
                r"^fad per common share - fully diluted$",
                1,
            ),
            (
                Metric::Affo,
                Unit::Total,
                r"^fad (?:available )?for common (?:stock and op unit holders|stockholders)$",
                1000,
            ),
            (
                Metric::Ffo,
                Unit::PerShare,
                r#"^(?:funds from operations \([“"]ffo[”"]\)|ffo) per (?:fully diluted )?common share (?:and|&) op units?(?:\s*[-–—]\s*fully diluted)?$"#,
                1,
            ),
        ],
        None,
    ))?;
    company.tables = tables;
    company.official_url = |url| {
        url.host_str() == Some("www.prnewswire.com")
            && url.path().starts_with("/news-releases/els-reports-")
    };
    company.affo_support = crate::AffoSupport::Reported;
    company.sections.push(affo_per_share);
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

fn affo_per_share(tables: &[crate::table::Table], source: &str) -> Result<Vec<crate::Observation>> {
    crate::affo_per_share(
        company()?,
        tables,
        source,
        crate::companies::Rule {
            metric: Metric::Affo,
            unit: Unit::Total,
            label: regex::Regex::new(
                r"^weighted average common (?:shares|stock) and op units? outstanding - fully diluted$",
            )?,
            scale: Some(1000),
            section: None,
        },
    )
}

fn tables(html: &str) -> Vec<crate::table::Table> {
    crate::table::joined_parentheses(html)
        .into_iter()
        .map(|mut table| {
            for cell in table.rows.iter_mut().take(8).flatten() {
                cell.text = cell.text.replace("Sept ", "September ");
            }
            table.rows.retain(|row| {
                !row.first().is_some_and(|cell| {
                    let label = cell.text.to_lowercase();
                    label.starts_with("fad for ") || label.starts_with("fad available for ")
                }) || row
                    .iter()
                    .skip(1)
                    .filter_map(|cell| crate::number(&cell.text))
                    .count()
                    > 1
            });
            table
        })
        .collect()
}
