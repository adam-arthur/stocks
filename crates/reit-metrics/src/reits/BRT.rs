use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    build((
        "BRT",
        14846,
        &["brtapartments.com", "www.brtapartments.com"],
        vec![
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^(?:nareit )?funds from operations per (?:diluted common share|common share basic and diluted|common share - diluted)(?: - note 2)?$",
                1,
            ),
            (
                Metric::Ffo,
                Unit::Total,
                r"^(?:nareit )?funds from operations(?: attributable to common (?:stockholders|shareholders))?$",
                1000,
            ),
        ],
        None,
    ))
}
