use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "CSR",
        798359,
        &[
            "www.centerspacehomes.com",
            "ir.centerspacehomes.com",
            "www.iret.com",
            "ir.iret.com",
        ],
        vec![
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^ffo per share and unit - diluted$",
                1,
            ),
            (Metric::Ffo, Unit::PerShare, r"^ffo - diluted$", 1),
            (Metric::Ffo, Unit::PerShare, r"^ffo per share$", 1),
        ],
        None,
    ))?;
    company.tables = crate::table::joined_parentheses;
    Ok(company)
}
