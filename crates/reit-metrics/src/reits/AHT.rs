use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    build((
        "AHT",
        1232582,
        &["www.ahtreit.com", "ahtreit.com"],
        vec![
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^ffo per diluted share(?: available to common stockholders and op unitholders)?$",
                1,
            ),
            (
                Metric::Ffo,
                Unit::Total,
                r"^ffo available to common stockholders and op unitholders$",
                1000,
            ),
        ],
        None,
    ))
}
