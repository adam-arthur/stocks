use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    build((
        "KIM",
        879101,
        &["www.kimcorealty.com", "investors.kimcorealty.com"],
        vec![
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^ffo per common share\s*[-–—]\s*diluted$",
                1,
            ),
            (
                Metric::Ffo,
                Unit::Total,
                r"^(?:ffo|funds from operations) available to (?:the company[\x27’]s )?common shareholders$",
                1000,
            ),
        ],
        None,
    ))
}
