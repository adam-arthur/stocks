use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    build((
        "ADC",
        917251,
        &[
            "investors.agreerealty.com",
            "www.agreerealty.com",
            "agreerealty.com",
        ],
        vec![
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^(?:ffo|funds from operations) per (?:common )?share(?: and (?:op|partnership) unit)?\s*[-–—]\s*diluted$",
                1,
            ),
            (
                Metric::Ffo,
                Unit::Total,
                r"^(?:ffo|funds from operations)\s*[-–—]\s*operating partnership common unitholders$",
                1000,
            ),
        ],
        None,
    ))
}
