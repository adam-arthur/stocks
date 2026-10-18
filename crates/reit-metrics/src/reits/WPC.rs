use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    build((
        "WPC",
        1025378,
        &["wpcarey.com", "www.wpcarey.com", "ir.wpcarey.com"],
        vec![
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^ffo (?:\(as defined by nareit\) )?attributable to w\. p\. carey (?:\(as defined by nareit\) )?per diluted share$",
                1,
            ),
            (
                Metric::Ffo,
                Unit::Total,
                r"^ffo (?:\(as defined by nareit\) )?attributable to w\. p\. carey(?: \(as defined by nareit\))?$",
                1000,
            ),
        ],
        None,
    ))
}
