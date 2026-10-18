use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    build((
        "KRG",
        1286043,
        &["www.kiterealty.com", "investor.kiterealty.com"],
        vec![
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^ffo, as defined by nareit, per share of the operating partnership\s*[-–—]\s*diluted$",
                1,
            ),
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^ffo per share of the operating partnership\s*[-–—]\s*diluted$",
                1,
            ),
            (
                Metric::Ffo,
                Unit::Total,
                r"^(?:ffo|funds from operations) attributable to (?:kite realty group trust )?common shareholders(?: 1)?$",
                1000,
            ),
        ],
        None,
    ))
}
