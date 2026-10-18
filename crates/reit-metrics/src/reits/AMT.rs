use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    build((
        "AMT",
        1053507,
        &["www.americantower.com", "investor.americantower.com"][..],
        vec![(
            Metric::Ffo,
            Unit::Total,
            r"^nareit (?:ffo attributable to amt common stockholders|funds from operations)$",
            0,
        )],
        None,
    ))
}
