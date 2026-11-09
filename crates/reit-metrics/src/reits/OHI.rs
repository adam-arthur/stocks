use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "OHI",
        888491,
        &["www.omegahealthcare.com", "omegahealthcare.com"],
        vec![
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^(?:nareit )?funds from operations available per share$",
                1,
            ),
            (Metric::Ffo, Unit::Total, r"^nareit ffo$", 1000),
        ],
        None,
    ))?;
    company.tables = crate::table::joined_parentheses;
    company.debt = Some(debt);
    Ok(company)
}

fn debt(html: &scraper::Html, as_of: chrono::NaiveDate) -> anyhow::Result<crate::debt::Debt> {
    if chrono::Datelike::month(&as_of) != 12 {
        anyhow::ensure!(
            !crate::debt::rows(html, as_of, "Total unsecured borrowings")?
                .iter()
                .any(|row| {
                    row.first()
                        .is_some_and(|label| label.starts_with("Total secured borrowings"))
                        && row
                            .iter()
                            .skip(1)
                            .find_map(|cell| crate::debt::amount(cell, 1000))
                            .is_none_or(|amount| amount != 0)
                }),
            "OHI secured debt requires a separate maturity schedule"
        );
        return crate::debt::bullets(
            html,
            as_of,
            "Total unsecured borrowings",
            (
                r"^(Revolving Credit Facility|20\d{2} (?:notes|Term Loan))",
                3,
                1,
            ),
            (
                "Total unsecured borrowings – net(3)(4)",
                &["Deferred financing costs", "Discount"],
            ),
        );
    }
    crate::debt::tables(
        html,
        as_of,
        &[crate::debt::Rule {
            anchor: "us-gaap:LongTermDebtMaturitiesRepaymentsOfPrincipalInYearTwo",
            ..crate::debt::Rule::new(0, 1000)
        }],
        None,
    )
}
