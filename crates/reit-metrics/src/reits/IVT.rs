use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "IVT",
        1307748,
        &[
            "www.inventrustproperties.com",
            "investor.inventrustproperties.com",
        ],
        vec![
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^(?:nareit )?ffo(?: applicable to common shares and dilutive securities)? per (?:diluted )?share(?:, diluted)?$",
                1,
            ),
            (
                Metric::Ffo,
                Unit::Total,
                r"^(?:nareit )?ffo applicable to common shares(?: and dilutive securities)?$",
                1000,
            ),
        ],
        None,
    ))?;
    company.filings = &["8-K", "8-K/A", "10-Q", "10-Q/A", "10-K", "10-K/A"];
    company.tables = crate::table::joined_parentheses;
    Ok(company)
}
