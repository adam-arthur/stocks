use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "AMT",
        1053507,
        &["www.americantower.com", "investor.americantower.com"][..],
        vec![
            (
                Metric::Affo,
                Unit::PerShare,
                r"^affo attributable to (?:amt |american tower corporation )?common stockholders per share$",
                1,
            ),
            (
                Metric::Affo,
                Unit::Total,
                r"^affo attributable to (?:amt |american tower corporation )?common stockholders$",
                1_000_000,
            ),
            (
                Metric::Ffo,
                Unit::Total,
                r"^nareit (?:ffo attributable to amt common stockholders|funds from operations)$",
                0,
            ),
        ],
        None,
    ))?;
    company.debt = Some(debt);
    company.ownership = Some(crate::ownership::Rules::default());
    company.affo_support = crate::AffoSupport::Reported;
    Ok(company)
}

fn debt(_html: &scraper::Html, _as_of: chrono::NaiveDate) -> anyhow::Result<crate::debt::Debt> {
    anyhow::bail!("maturity schedule combines borrowings with finance leases")
}
