use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "AAT",
        1500217,
        &["ir.americanassetstrust.com", "www.americanassetstrust.com"],
        vec![
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^ffo per diluted share/unit$",
                1,
            ),
            (
                Metric::Ffo,
                Unit::Total,
                r"^ffo attributable to common stock and common units$",
                1000,
            ),
        ],
        None,
    ))?;
    company.debt = Some(debt);
    Ok(company)
}

fn debt(html: &scraper::Html, as_of: chrono::NaiveDate) -> anyhow::Result<crate::debt::Debt> {
    if chrono::Datelike::month(&as_of) != 12 {
        let text =
            crate::table::normalize(&html.root_element().text().collect::<Vec<_>>().join(" "));
        anyhow::ensure!(
            text.contains("(1) Interest only."),
            "AAT secured repayment terms changed"
        );
        anyhow::ensure!(
            regex::Regex::new(&format!(
                r"As of {},[^.]*there were no amounts outstanding under the \d{{4}} Revolver Loan",
                as_of.format("%B %-d, %Y")
            ))?
            .is_match(&text),
            "AAT revolver balance is not confirmed zero"
        );
        let secured = crate::debt::bullets(
            html,
            as_of,
            "Total Secured Notes Payable Outstanding",
            (r"^City Center Bellevue", 1, 4),
            (
                "Total Secured Notes Payable Outstanding",
                &["Debt issuance costs, net"],
            ),
        )?;
        let unsecured = crate::debt::bullets(
            html,
            as_of,
            "Total Unsecured Notes Payable",
            (
                r"^(Term Loan A|Senior Guaranteed Notes, Series [A-Z]|[\d.]+% Senior Notes)$",
                1,
                4,
            ),
            (
                "Total Unsecured Notes Payable",
                &["Debt discount and issuance costs"],
            ),
        )?;
        let debt = crate::debt::add_payments(
            secured,
            &unsecured.maturities.into_iter().collect::<Vec<_>>(),
        )?;
        return crate::debt::add_payments(
            debt,
            &[(chrono::Datelike::year(&as_of) + 5, unsecured.thereafter)],
        );
    }
    crate::debt::tables(html, as_of, &[crate::debt::Rule::new(0, 1000)], None)
}
