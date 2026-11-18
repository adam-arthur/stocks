use crate::companies::{Company, build};
use anyhow::{Context, Result, ensure};

pub fn company() -> Result<Company> {
    let mut company = build(("SRG", 1628063, &[], vec![], None))?;
    company.affo_support = crate::AffoSupport::Unavailable;
    company.debt = Some(|html, as_of| {
        let text =
            crate::table::normalize(&html.root_element().text().collect::<Vec<_>>().join(" "));
        let pattern = regex::Regex::new(&format!(
            r"Term Loan Facility principal balance to \$\s*([\d,.]+) million as of {}\s*\. The maturity date for the Term Loan Facility is ([A-Za-z]+ \d{{1,2}}, 20\d{{2}})",
            as_of.format("%B %-d, %Y")
        ))?;
        let matched = pattern
            .captures(&text)
            .context("missing complete term loan repayment disclosure")?;
        let maturity = chrono::NaiveDate::parse_from_str(&matched[2], "%B %d, %Y")?;
        use chrono::Datelike;
        ensure!(
            maturity > as_of && maturity.year() == (as_of.succ_opt().unwrap()).year(),
            "term loan spans multiple payment years"
        );
        let principal =
            crate::debt::amount(&matched[1], 1_000_000).context("invalid term loan principal")?;
        crate::debt::payments(as_of, &[(maturity.year(), principal)])
    });
    Ok(company)
}
