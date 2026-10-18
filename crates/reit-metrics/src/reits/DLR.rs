use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    build((
        "DLR",
        1297996,
        &["www.digitalrealty.com", "investor.digitalrealty.com"],
        vec![
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^funds from operations per share\s*[-–—]\s*diluted$",
                1,
            ),
            (Metric::Ffo, Unit::Total, r"^funds from operations$", 1000),
        ],
        None,
    ))
}
