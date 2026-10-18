use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "CHCT",
        1631569,
        &["www.chct.reit", "investors.chct.reit"],
        vec![
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^(?:funds from operations|ffo) per common share(?:\s*-\s*diluted)?$",
                1,
            ),
            (Metric::Ffo, Unit::Total, r"^funds from operations$", 1000),
        ],
        None,
    ))?;
    company.tables = crate::table::joined_parentheses;
    Ok(company)
}
