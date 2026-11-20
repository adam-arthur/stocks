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
                0,
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
    company.tables = |html| {
        crate::table::tables(html)
            .into_iter()
            .filter(|table| {
                !table
                    .rows
                    .iter()
                    .take(8)
                    .flatten()
                    .any(|cell| cell.text.to_ascii_lowercase().starts_with("growth rate"))
            })
            .collect()
    };
    company.debt = Some(debt);
    company.ownership = Some(crate::ownership::Rules::default());
    company.affo_support = crate::AffoSupport::Reported;
    Ok(company)
}

fn debt(html: &scraper::Html, as_of: chrono::NaiveDate) -> Result<crate::debt::Debt> {
    use crate::debt::{Rule, scalar, tables};
    use anyhow::{Context, ensure};

    let leases = scalar(html, "us-gaap:FinanceLeaseLiability", as_of)?
        .context("missing consolidated finance lease balance")?;
    let rule = if leases == 0 {
        Rule {
            anchor: "amt:LongTermDebtAndLeaseObligationMaturityYearOne",
            ..Rule::new(0, 1_000_000)
        }
    } else {
        Rule::new(0, 1_000_000)
    };
    let debt = tables(html, as_of, &[rule], None).with_context(|| {
        format!("no complete borrowing principal schedule excluding ${leases} of finance leases")
    })?;
    let gross = scalar(html, "amt:LongTermDebtAndLeaseObligationGross", as_of)?
        .context("missing gross debt and lease principal")?;
    ensure!(
        leases >= 0 && gross.checked_sub(leases) == Some(i64::try_from(debt.total_debt_principal)?),
        "maturities do not reconcile to gross borrowing principal excluding finance leases"
    );
    Ok(debt)
}
