use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    build((
        "EQIX",
        1101239,
        &["investor.equinix.com", "www.equinix.com"][..],
        vec![
            (
                Metric::Ffo,
                Unit::Total,
                r"^ffo(?: attributable to common stockholders)?$",
                0,
            ),
            (Metric::Ffo, Unit::PerShare, r"^diluted ffo per share$", 1),
        ],
        None,
    ))
}
