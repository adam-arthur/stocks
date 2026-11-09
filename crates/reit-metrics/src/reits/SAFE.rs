use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "SAFE",
        1095651,
        &[
            "www.safeholdinc.com",
            "safeholdinc.com",
            "ir.safeholdinc.com",
        ],
        vec![
            (
                Metric::Ffo,
                Unit::Total,
                r"^ffo allocable to (?:safety, income & growth|safehold) inc\. common shareholders$",
                1000,
            ),
            (Metric::Ffo, Unit::Total, r"^ffo$", 1000),
        ],
        None,
    ))?;
    company.filings = &["8-K", "8-K/A", "10-Q", "10-Q/A", "10-K", "10-K/A"];
    company.tables = tables;
    company.official_url = |url| {
        url.host_str() == Some("www.sec.gov")
            && url.path().starts_with("/Archives/edgar/data/1688852/")
    };
    company.supplements = Some(|sources, _, since, _| Box::pin(supplements(sources, since)));
    company.supplement_period_required = true;
    company.debt = Some(debt);
    Ok(company)
}

async fn supplements(
    sources: &crate::source::Sources,
    since: chrono::NaiveDate,
) -> Result<Vec<(chrono::NaiveDate, String)>> {
    let mut predecessor = company()?;
    predecessor.cik = 1688852;
    let mut result = Vec::new();
    for filing in sources
        .sec
        .earnings_filings(predecessor.cik, since)
        .await?
        .into_iter()
        .filter(|filing| matches!(filing.form.as_str(), "10-Q" | "10-Q/A" | "10-K" | "10-K/A"))
    {
        for url in sources
            .filing_urls(&predecessor, &filing.accession, &filing.form)
            .await?
        {
            let extracted =
                crate::extract_file("SAFE", &sources.download(&predecessor, &url).await?, &url)?;
            if let Some(end) = extracted.observations.iter().map(|value| value.end).max()
                && end >= since
                && end <= filing.filed_on
            {
                result.push((end, url));
            }
        }
    }
    Ok(result)
}

fn tables(html: &str) -> Vec<crate::table::Table> {
    crate::table::joined_parentheses(html)
        .into_iter()
        .map(|mut table| {
            let mut headers = std::collections::BTreeSet::new();
            for row in table.rows.iter_mut().take(8) {
                for cell in row.iter_mut() {
                    cell.text = cell.text.replace("For the Period from", "For the Period");
                }
                row.retain(|cell| {
                    (crate::duration(&cell.text).is_none()
                        && !cell.text.to_lowercase().starts_with("for the period"))
                        || headers.insert((cell.text.clone(), cell.column, cell.width))
                });
            }
            table
        })
        .collect()
}

fn debt(html: &scraper::Html, as_of: chrono::NaiveDate) -> anyhow::Result<crate::debt::Debt> {
    crate::debt::tables(html, as_of, &[crate::debt::Rule::new(2, 1000)], None)
}
