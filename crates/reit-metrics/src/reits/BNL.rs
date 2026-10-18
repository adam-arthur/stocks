use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    build((
        "BNL",
        1424182,
        &[
            "investors.broadstone.com",
            "broadstone.com",
            "www.broadstone.com",
        ],
        vec![
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^ffo per (?:diluted )?share(?:, (?:basic and )?diluted)?(?:\s+\d+)?$",
                1,
            ),
            (Metric::Ffo, Unit::Total, r"^ffo$", 1000),
        ],
        None,
    ))
}
