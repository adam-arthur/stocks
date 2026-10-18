use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    build((
        "XHR",
        1616000,
        &[
            "xeniareit.com",
            "www.xeniareit.com",
            "investors.xeniareit.com",
        ],
        vec![(
            Metric::Ffo,
            Unit::Total,
            r"^ffo attributable to common stock and unit holders$",
            1000,
        )],
        None,
    ))
}
