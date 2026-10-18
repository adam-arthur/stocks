use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    build((
        "UE",
        1611547,
        &["uedge.com", "www.uedge.com", "investors.uedge.com"],
        vec![(
            Metric::Ffo,
            Unit::PerShare,
            r"^ffo per diluted (?:common )?share$",
            1,
        )],
        None,
    ))
}
