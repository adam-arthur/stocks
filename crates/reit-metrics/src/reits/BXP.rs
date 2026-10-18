use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    build((
        "BXP",
        1037540,
        &["www.bxp.com", "ir.bxp.com", "ir.bostonproperties.com"],
        vec![
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^(?:diluted ffo per share|ffo per share(?: -)? diluted)(?:\s+\d+)?$",
                1,
            ),
            (
                Metric::Ffo,
                Unit::Total,
                r"^(?:ffo|funds from operations) attributable to (?:boston properties|bxp), inc\.(?: common shareholders)?(?:\s+\d+)?$",
                1000,
            ),
        ],
        None,
    ))
}
