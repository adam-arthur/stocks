use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::{Context, Result, ensure};

pub fn company() -> Result<Company> {
    let mut company = build((
        "XRN",
        1533615,
        &[],
        vec![
            (
                Metric::Ffo,
                Unit::Total,
                r"^ffo attributable to common stockholders and noncontrolling interest$",
                1000,
            ),
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^ffo attributable to common stockholders and noncontrolling interest per share and unit$",
                1,
            ),
            (
                Metric::Affo,
                Unit::PerShare,
                r"^affo(?: attributable to common stockholders and noncontrolling interest)? per share and unit$",
                1,
            ),
            (
                Metric::Affo,
                Unit::Total,
                r"^affo(?: attributable to common stockholders and noncontrolling interest)?$",
                1000,
            ),
            (
                Metric::Affo,
                Unit::Total,
                r"^fad attributable to common stockholders and noncontrolling interest$",
                1000,
            ),
        ],
        None,
    ))?;
    company.affo_support = crate::AffoSupport::Reported;
    company.debt = Some(debt);
    Ok(company)
}

fn debt(html: &scraper::Html, as_of: chrono::NaiveDate) -> Result<crate::debt::Debt> {
    let text = crate::table::normalize(&html.root_element().text().collect::<Vec<_>>().join(" "));
    ensure!(
        text.contains("no Notes had been issued or were outstanding"),
        "XRN senior notes require review"
    );
    let rows = crate::debt::rows(html, as_of, "Credit Facility, gross")?;
    let balance = |label| {
        rows.iter()
            .find(|row| row.first().is_some_and(|first| first == label))
            .and_then(|row| row.get(1))
            .and_then(|value| crate::debt::amount(value, 1000))
            .with_context(|| format!("missing {label} balance"))
    };
    let mut payments = Vec::new();
    for matched in
        regex::Regex::new(r"\$\s*([\d,.]+) million term loan maturing in [A-Za-z]+ (20\d{2})")?
            .captures_iter(&text)
    {
        payments.push((
            matched[2].parse()?,
            crate::debt::amount(&matched[1], 1_000_000).context("invalid term loan amount")?,
        ));
    }
    ensure!(
        payments.len() == 4
            && payments[..3].iter().map(|(_, amount)| amount).sum::<u64>()
                == balance("Term Loan A Tranches")?
            && payments[3].1 == balance("Term Loan B")?,
        "XRN credit facility tranches do not reconcile"
    );
    let revolver = regex::Regex::new(r"million revolver maturing in [A-Za-z]+ (20\d{2})")?
        .captures(&text)
        .context("missing revolver maturity")?;
    payments.push((revolver[1].parse()?, balance("Revolver")?));
    ensure!(
        payments.iter().map(|(_, amount)| amount).sum::<u64>()
            == balance("Credit Facility, gross")?,
        "XRN credit facility does not reconcile"
    );
    let note = crate::debt::tables(
        html,
        as_of,
        &[crate::debt::Rule {
            anchor: "",
            intro: "scheduled principal payments due for each year ended December 31",
            ..crate::debt::Rule::new(0, 1000)
        }],
        None,
    )?;
    crate::debt::add_payments(note, &payments)
}
