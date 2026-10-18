use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    build((
        "STAG",
        1479094,
        &["www.stagindustrial.com", "ir.stagindustrial.com"],
        vec![
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^(?:ffo|funds from operations) per share\s*/\s*unit\s*[-–—]\s*diluted$",
                1,
            ),
            (
                Metric::Ffo,
                Unit::Total,
                r"^funds from operations attributable to common stockholders and unit holders$",
                1000,
            ),
        ],
        None,
    ))
}
