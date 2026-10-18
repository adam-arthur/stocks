use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    build((
        "BFS",
        907254,
        &[
            "ir.saulcenters.com",
            "www.saulcenters.com",
            "saulcenters.com",
        ],
        vec![
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^(?:(?:basic and )?diluted )?ffo per share available to common stockholders and noncontrolling interests$",
                1,
            ),
            (
                Metric::Ffo,
                Unit::Total,
                r"^ffo available to common stockholders and noncontrolling interests$",
                1000,
            ),
        ],
        None,
    ))
}
