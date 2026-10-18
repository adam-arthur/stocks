use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    build((
        "VTR",
        740260,
        &["ventasreit.com", "www.ventasreit.com", "ir.ventasreit.com"],
        vec![(
            Metric::Ffo,
            Unit::PerShare,
            r"^(?:nareit )?ffo attributable to common stockholders per share$",
            1,
        )],
        None,
    ))
}
