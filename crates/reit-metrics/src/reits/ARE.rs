use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    build((
        "ARE",
        1035443,
        &["www.are.com", "investor.are.com"],
        vec![
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^(?:ffo|funds from operations) per share attributable to alexandria['’]s common stockholders [–—-] diluted$",
                1,
            ),
            (
                Metric::Ffo,
                Unit::Total,
                r"^(?:ffo|funds from operations) attributable to alexandria['’]s common stockholders [–—-] diluted$",
                1000,
            ),
        ],
        Some("per share attributable to alexandria"),
    ))
}
