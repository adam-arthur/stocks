use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "UDR",
        74208,
        &["udr.com", "www.udr.com", "ir.udr.com"],
        vec![
            (
                Metric::Affo,
                Unit::PerShare,
                r"^affo per (?:weighted average )?common share and unit, diluted$",
                1,
            ),
            (
                Metric::Affo,
                Unit::Total,
                r"^affo attributable to common stockholders and unitholders(?:, diluted)?$",
                1000,
            ),
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^ffo per (?:weighted average )?common share and unit, diluted$",
                1,
            ),
        ],
        None,
    ))?;
    company.pdf = Some(crate::pdf::Rules {
        page: pdf_page,
        total_scale: 1000,
        metric: pdf_metric,
        current_only,
    });
    company.debt = Some(debt);
    company.affo_support = crate::AffoSupport::Reported;
    Ok(company)
}

pub(crate) fn pdf_page(text: &str) -> bool {
    text.contains("fundsfromoperations")
        && (text.contains("ffoperweightedaveragecommonshareandunit,diluted")
            || text.contains("ffopercommonshareandunit,diluted"))
        && text.contains("inthousands,exceptpershareandunitamounts")
}

pub(crate) fn pdf_metric(label: &str, _: &str) -> Option<(Metric, Unit)> {
    match label {
        "ffoperweightedaveragecommonshareandunit,diluted" | "ffopercommonshareandunit,diluted" => {
            Some((Metric::Ffo, Unit::PerShare))
        }
        "affoperweightedaveragecommonshareandunit,diluted"
        | "affopercommonshareandunit,diluted" => Some((Metric::Affo, Unit::PerShare)),
        "affoattributabletocommonstockholdersandunitholders,diluted" => {
            Some((Metric::Affo, Unit::Total))
        }
        _ => None,
    }
}

fn current_only(line: &str, year_row: &str, numbers: usize, years: usize) -> bool {
    let numeric = regex::Regex::new(r"(\(?-?\d[\d,]*(?:\.\d+)?\)?)\s*(%?)").unwrap();
    let year = regex::Regex::new(r"\b20\d{2}\b").unwrap();
    numbers * 2 == years
        && line
            .chars()
            .any(|value| value.is_control() && !value.is_whitespace())
        && numeric
            .captures_iter(line)
            .zip(
                year.find_iter(year_row)
                    .collect::<Vec<_>>()
                    .as_chunks::<2>()
                    .0
                    .iter(),
            )
            .all(|(value, years)| {
                value.get(1).is_some_and(|value| {
                    value.start() >= years[0].start() && value.end() <= years[1].start()
                })
            })
}

fn debt(html: &scraper::Html, as_of: chrono::NaiveDate) -> anyhow::Result<crate::debt::Debt> {
    crate::debt::tables(html, as_of, &[crate::debt::Rule::new(2, 1000)], None)
}
