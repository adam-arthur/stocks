use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "ESRT",
        1541401,
        &[
            "investors.empirestaterealtytrust.com",
            "investors.esrtreit.com",
        ],
        vec![
            (Metric::Ffo, Unit::PerShare, r#"^ffo per share diluted$"#, 1),
            (
                Metric::Ffo,
                Unit::Total,
                r#"^ffo attributable to common stockholders and operating partnership units$"#,
                1000,
            ),
        ],
        None,
    ))?;
    company.tables = crate::table::joined_parentheses;
    Ok(company)
}
