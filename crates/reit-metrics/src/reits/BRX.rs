use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    build((
        "BRX",
        1581068,
        &["www.brixmor.com", "investors.brixmor.com"],
        vec![(
            Metric::Ffo,
            Unit::PerShare,
            r"^nareit ffo per (?:diluted share|share/op unit - diluted)$",
            1,
        )],
        None,
    ))
}
