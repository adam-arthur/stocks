use anyhow::{Context, Result, ensure};
use chrono::NaiveDate;

use crate::companies::{Company, build};
use crate::{Metric, Unit};

pub fn company() -> Result<Company> {
    let mut company = build((
        "BSRT",
        1777917,
        &["investors.bsrreit.com", "www.bsrreit.com", "bsrreit.com"],
        vec![
            (Metric::Ffo, Unit::PerShare, r"^ffo per unit\s*\d*$", 1),
            (
                Metric::Ffo,
                Unit::Total,
                r#"^(?:ffo|funds from operations \("ffo"\))\s*\d*$"#,
                1000,
            ),
        ],
        None,
    ))?;
    company.tables = tables;
    company.filings = &[];
    company.supplements = Some(|_, company, since, _| Box::pin(supplements(company, since)));
    company.supplement_period_required = true;
    Ok(company)
}

async fn supplements(company: &Company, since: NaiveDate) -> Result<Vec<(NaiveDate, String)>> {
    let feed: serde_json::Value = reqwest::Client::builder()
        .user_agent(format!(
            "stocks-reit-metrics {}",
            std::env::var("SEC_CONTACT_EMAIL")?
        ))
        .timeout(std::time::Duration::from_secs(90))
        .redirect(reqwest::redirect::Policy::none())
        .build()?
        .get("https://investors.bsrreit.com/feed/PressRelease.svc/GetPressReleaseList")
        .query(&[
            ("LanguageId", "1"),
            ("bodyType", "0"),
            ("pressReleaseDateFilter", "1"),
            ("categoryId", "00000000-0000-0000-0000-000000000000"),
            ("pageSize", "-1"),
            ("pageNumber", "0"),
            ("year", "-1"),
            ("excludeSelection", "1"),
        ])
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let pattern = regex::Regex::new(
        r"(?i)(?:(first|second|third|fourth) quarter(?: and full year)?|q([1-4])) (20\d{2})",
    )?;
    let mut result = Vec::new();
    for release in feed["GetPressReleaseListResult"]
        .as_array()
        .context("missing BSR news archive")?
    {
        let title = release["Headline"]
            .as_str()
            .unwrap_or_default()
            .to_lowercase();
        if !title.contains("announces") || !title.contains("financial results") {
            continue;
        }
        let (year, month) = if let Some(period) = pattern.captures(&title) {
            (
                period[3].parse()?,
                match period.get(1).map(|value| value.as_str()) {
                    Some("first") => 3,
                    Some("second") => 6,
                    Some("third") => 9,
                    Some("fourth") => 12,
                    _ => period[2].parse::<u32>()? * 3,
                },
            )
        } else if title.contains("strong 2020 financial results") {
            (2020, 12)
        } else {
            anyhow::bail!("unrecognized BSR earnings release: {title}");
        };
        let end =
            NaiveDate::from_ymd_opt(year, month, if matches!(month, 6 | 9) { 30 } else { 31 })
                .context("invalid BSR report period")?;
        if end >= since && end <= chrono::Utc::now().date_naive() {
            let url = reqwest::Url::parse("https://investors.bsrreit.com")?.join(
                release["LinkToDetailPage"]
                    .as_str()
                    .context("missing BSR release URL")?,
            )?;
            crate::source::validate_url(company, url.as_str())?;
            result.push((end, url.to_string()));
        }
    }
    ensure!(
        !result.is_empty(),
        "BSR archive contains no matching earnings releases"
    );
    result.sort();
    result.dedup();
    Ok(result)
}

fn tables(html: &str) -> Vec<crate::table::Table> {
    let mut tables = crate::table::tables(html);
    let quarter = regex::Regex::new(r"^Q([1-4]) (20\d{2})$").unwrap();
    let annual = regex::Regex::new(r"^(FY|YTD) (20\d{2})$").unwrap();
    let decimal = regex::Regex::new(r"^\$?\s*\(?-?\d+,\d{2}\)?$").unwrap();
    let latest = tables
        .iter()
        .flat_map(|table| table.rows.iter().take(3).flatten())
        .filter_map(|cell| quarter.captures(&cell.text))
        .map(|period| {
            (
                period[2].parse::<i32>().unwrap(),
                period[1].parse::<u32>().unwrap(),
            )
        })
        .max();
    for table in &mut tables {
        for index in 0..table.rows.len().saturating_sub(1) {
            let (before, after) = table.rows.split_at_mut(index + 1);
            for cell in &mut before[index] {
                if ["March", "June", "September", "December"]
                    .iter()
                    .any(|month| cell.text.ends_with(month))
                    && let Some(next) = after[0].iter_mut().find(|next| {
                        next.column == cell.column
                            && (next.text.starts_with("30,") || next.text.starts_with("31,"))
                    })
                {
                    cell.text = format!("{} {}", cell.text, next.text);
                    next.text.clear();
                }
            }
        }
        for row in &mut table.rows {
            let per_unit = row
                .iter()
                .find(|cell| !cell.text.is_empty())
                .is_some_and(|cell| cell.text.to_lowercase().starts_with("ffo per unit"));
            for cell in row {
                if per_unit && decimal.is_match(&cell.text) {
                    cell.text = cell.text.replace(',', ".");
                }
                cell.text = cell.text.replace("monthsended", "months ended");
                if let Some(period) = annual.captures(&cell.text) {
                    let end = if &period[1] == "FY" {
                        Some(("Twelve", "December 31"))
                    } else {
                        latest.map(|(_, quarter)| match quarter {
                            1 => ("Three", "March 31"),
                            2 => ("Six", "June 30"),
                            3 => ("Nine", "September 30"),
                            _ => ("Twelve", "December 31"),
                        })
                    };
                    if let Some((months, end)) = end {
                        cell.text = format!("{months} months ended {end}, {}", &period[2]);
                    }
                }
            }
        }
    }
    tables
}
