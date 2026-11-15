use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "PSA",
        1393311,
        &["investors.publicstorage.com"][..],
        vec![
            (Metric::Affo, Unit::Total, r"^fad$", 1000),
            (
                Metric::Affo,
                Unit::PerShare,
                r"^fad per share(?: and unit)?$",
                1,
            ),
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^ffo per share(?: and unit)?$",
                1,
            ),
            (
                Metric::Ffo,
                Unit::Total,
                r"^ffo allocable to common shares(?: and units)?$",
                1000,
            ),
        ],
        Some("reconciliation of ffo to fad"),
    ))?;
    company.sections.push(affo_per_share);
    company.affo_support = crate::AffoSupport::Reported;
    company.official_url =
        |url| url.host_str() == Some("s1.q4cdn.com") && url.path().starts_with("/588671402/");
    company.pdf = Some(crate::pdf::Rules {
        page: pdf_page,
        total_scale: 1000,
        metric: pdf_metric,
        current_only: |_, _, _, _| false,
    });
    company.debt = Some(debt);
    Ok(company)
}

pub fn pdf_page(text: &str) -> bool {
    text.contains("reconciliationofffo")
}

pub fn pdf_metric(label: &str, _: &str) -> Option<(Metric, Unit)> {
    match label.trim_end_matches('*') {
        "ffopershare(a)" | "ffopershare" | "ffopershareandunit(a)" | "ffopershareandunit" => {
            Some((Metric::Ffo, Unit::PerShare))
        }
        "fad(a)" | "fad" => Some((Metric::Affo, Unit::Total)),
        "fadpershare(a)" | "fadpershare" | "fadpershareandunit(a)" | "fadpershareandunit" => {
            Some((Metric::Affo, Unit::PerShare))
        }
        _ => None,
    }
}

fn debt(html: &scraper::Html, as_of: chrono::NaiveDate) -> anyhow::Result<crate::debt::Debt> {
    crate::debt::tables(html, as_of, &[crate::debt::Rule::new(2, 1000)], None)
}

fn affo_per_share(tables: &[crate::table::Table], source: &str) -> Result<Vec<crate::Observation>> {
    let values = crate::affo_per_share(
        company()?,
        tables,
        source,
        crate::companies::Rule {
            metric: Metric::Affo,
            unit: Unit::Total,
            label: regex::Regex::new(
                r"^diluted weighted average common shares and units outstanding$",
            )?,
            scale: Some(1000),
            section: None,
        },
    )?;
    Ok(values)
}
