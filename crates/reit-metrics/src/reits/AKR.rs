use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    build((
        "AKR",
        899629,
        &["ir.acadiarealty.com", "www.acadiarealty.com"],
        vec![
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^(?:diluted )?funds from operations, per common share and common op unit$",
                1,
            ),
            (
                Metric::Ffo,
                Unit::Total,
                r"^funds from operations attributable to common shareholders and common op unit holders(?: - diluted)?$",
                1000,
            ),
        ],
        None,
    ))
}
