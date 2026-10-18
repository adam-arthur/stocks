use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "AHR",
        1632970,
        &[
            "ir.americanhealthcarereit.com",
            "www.americanhealthcarereit.com",
        ],
        vec![
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^(?:nareit )?ffo per common share attributable to controlling interest\s*[-–—]\s*(?:basic and )?diluted$",
                1,
            ),
            (
                Metric::Ffo,
                Unit::Total,
                r"^(?:nareit )?ffo attributable to controlling interest$",
                1000,
            ),
        ],
        None,
    ))?;
    company.additional_items = Some(|items| items.split(',').any(|item| item.trim() == "7.01"));
    Ok(company)
}
