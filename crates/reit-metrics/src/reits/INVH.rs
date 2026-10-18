use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    build((
        "INVH",
        1687229,
        &["www.invitationhomes.com", "ir.invitationhomes.com"],
        vec![(
            Metric::Ffo,
            Unit::PerShare,
            r"^ffo per share\s*[-–—]\s*diluted$",
            1,
        )],
        None,
    ))
}
