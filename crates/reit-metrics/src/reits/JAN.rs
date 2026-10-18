use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    build((
        "JAN",
        2100805,
        &["www.janusreit.com", "ir.janusreit.com"],
        vec![
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^diluted nareit ffo per common share$",
                1,
            ),
            (Metric::Ffo, Unit::Total, r"^diluted nareit ffo$", 1000),
        ],
        None,
    ))
}
