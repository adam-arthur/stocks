use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    build((
        "AAT",
        1500217,
        &["ir.americanassetstrust.com", "www.americanassetstrust.com"],
        vec![
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^ffo per diluted share/unit$",
                1,
            ),
            (
                Metric::Ffo,
                Unit::Total,
                r"^ffo attributable to common stock and common units$",
                1000,
            ),
        ],
        None,
    ))
}
