use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "PSA",
        1393311,
        &["investors.publicstorage.com"][..],
        vec![
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
    company.official_url =
        |url| url.host_str() == Some("s1.q4cdn.com") && url.path().starts_with("/588671402/");
    company.pdf = Some(crate::pdf::Rules {
        page: pdf_page,
        total_scale: 1000,
        metric: pdf_metric,
        current_only: |_, _, _, _| false,
    });
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
        _ => None,
    }
}
