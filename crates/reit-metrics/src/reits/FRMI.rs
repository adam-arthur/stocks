use crate::companies::{Company, build};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build(("FRMI", 2071778, &[], vec![], None))?;
    company.affo_support = crate::AffoSupport::Unavailable;
    company.debt = Some(|html, as_of| {
        crate::debt::tables(
            html,
            as_of,
            &[crate::debt::Rule {
                anchor: "",
                intro: "annual principal maturities of outstanding debt obligations",
                ..crate::debt::Rule::new(0, 1000)
            }],
            None,
        )
    });
    Ok(company)
}
