use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    build((
        "PINE",
        1786117,
        &["alpinereit.com", "www.alpinereit.com", "ir.alpinereit.com"][..],
        vec![
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^ffo (?:attributable to common stockholders )?per diluted share$",
                1,
            ),
            (
                Metric::Ffo,
                Unit::Total,
                r"^ffo attributable to common stockholders$",
                1000,
            ),
        ],
        None,
    ))
}
