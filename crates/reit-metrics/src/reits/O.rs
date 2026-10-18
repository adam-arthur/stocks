use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    build((
        "O",
        726728,
        &["www.realtyincome.com"][..],
        vec![
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^ffo per (?:diluted )?share$",
                1,
            ),
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^ffo per common share:? diluted$",
                1,
            ),
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^ffo per common share, basic and diluted$",
                1,
            ),
            (
                Metric::Ffo,
                Unit::Total,
                r"^funds from operations available to common stockholders \(ffo\)$",
                0,
            ),
        ],
        None,
    ))
}
