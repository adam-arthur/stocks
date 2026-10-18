use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    build((
        "JBGS",
        1689796,
        &["www.jbgsmith.com", "investors.jbgsmith.com"],
        vec![
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^ffo per (?:common share\s*[-–—]\s*diluted|diluted common share)$",
                1,
            ),
            (
                Metric::Ffo,
                Unit::Total,
                r"^ffo attributable to common shareholders$",
                1000,
            ),
        ],
        None,
    ))
}
