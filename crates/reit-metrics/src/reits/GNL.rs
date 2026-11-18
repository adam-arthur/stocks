use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "GNL",
        1526113,
        &[],
        vec![
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^(?:nareit defined )?ffo per (?:diluted )?common share$",
                1,
            ),
            (
                Metric::Ffo,
                Unit::Total,
                r"^ffo \((?:as )?defined by nareit\)(?: attributable to (?:common )?stockholders)?(?: \[1\])?$",
                1000,
            ),
        ],
        None,
    ))?;
    company.extra_observations = Some(highlights);
    company.tables = crate::table::joined_parentheses;
    company.rules.extend([
        crate::companies::Rule { metric: Metric::Affo, unit: Unit::PerShare, label: regex::Regex::new(r"(?i)^affo per diluted common share$")?, scale: Some(1), section: None },
        crate::companies::Rule { metric: Metric::Affo, unit: Unit::Total, label: regex::Regex::new(r"(?i)^(?:adjusted funds from operations attributable to common stockholders|affo(?: attributable to (?:common )?stockholders)?)$")?, scale: Some(1000), section: None },
    ]);
    company.affo_support = crate::AffoSupport::Reported;
    company.debt = Some(debt);
    Ok(company)
}

fn highlights(html: &str, source: &str) -> Result<Vec<crate::Observation>> {
    let text = crate::table::normalize(
        &scraper::Html::parse_document(html)
            .root_element()
            .text()
            .collect::<Vec<_>>()
            .join(" "),
    );
    let quarter = regex::Regex::new(
        r"(?i)financial and operating results for the quarter ended ([a-z]+ [0-9]+, 20[0-9]{2})",
    )?;
    let Some(period) = quarter.captures(&text) else {
        return Ok(Vec::new());
    };
    let end = chrono::NaiveDate::parse_from_str(&period[1], "%B %d, %Y")?;
    let value = regex::Regex::new(
        r#"(?i)NAREIT defined Funds from Operations \([“"]FFO[”"]\) of \$[0-9.]+ million, or \$([0-9.]+) per share"#,
    )?;
    let Some(value) = value.captures(&text[period.get(0).unwrap().end()..]) else {
        return Ok(Vec::new());
    };
    Ok(vec![crate::Observation {
        metric: Metric::Ffo,
        unit: Unit::PerShare,
        label: "NAREIT defined FFO per share".into(),
        scale: 1,
        value: value[1].parse()?,
        published_on: None,
        start: end
            .succ_opt()
            .unwrap()
            .checked_sub_months(chrono::Months::new(3))
            .unwrap(),
        end,
        source: source.into(),
        location: "quarterly operating results".into(),
    }])
}

fn debt(html: &scraper::Html, as_of: chrono::NaiveDate) -> anyhow::Result<crate::debt::Debt> {
    if chrono::Datelike::month(&as_of) != 12 {
        use anyhow::Context;
        let text =
            crate::table::normalize(&html.root_element().text().collect::<Vec<_>>().join(" "));
        let maturity = |pattern: &str| -> anyhow::Result<i32> {
            let years = regex::Regex::new(pattern)?
                .captures_iter(&text)
                .map(|matched| matched[1].parse::<i32>())
                .collect::<std::result::Result<std::collections::BTreeSet<_>, _>>()?;
            anyhow::ensure!(years.len() == 1, "missing or conflicting GNL maturity");
            Ok(*years.first().unwrap())
        };
        let debt = crate::debt::tables(html, as_of, &[crate::debt::Rule::new(0, 1000)], None)?;
        let mortgage_rows = crate::debt::rows(html, as_of, "Gross mortgage notes payable")?;
        anyhow::ensure!(
            mortgage_rows
                .iter()
                .find(|row| row
                    .first()
                    .is_some_and(|label| label == "Gross mortgage notes payable"))
                .and_then(|row| row.get(2))
                .and_then(|value| crate::debt::amount(value, 1000))
                == Some(debt.total_debt_principal),
            "GNL mortgage principal does not reconcile"
        );
        let mut payments = vec![(
            maturity(r"Revolving Credit Facility matures on [A-Za-z]+ \d+, (20\d{2})")?,
            u64::try_from(
                crate::debt::scalar(html, "us-gaap:LineOfCredit", as_of)?
                    .context("missing GNL revolver balance")?,
            )?,
        )];
        let rows = crate::debt::rows(html, as_of, "Senior Notes, Net")?;
        let mut principal = 0u64;
        let mut adjustments = 0u64;
        let mut rate = None;
        let mut seen = std::collections::BTreeSet::new();
        for row in &rows {
            let Some(label) = row.first() else { continue };
            if matches!(label.as_str(), "3.75% Senior Notes" | "4.50% Senior Notes") {
                rate = Some(label[..4].to_string());
            } else if label == "Aggregate principal amount" {
                let rate = rate.take().context("unrecognized GNL note series")?;
                anyhow::ensure!(seen.insert(rate.clone()), "duplicate GNL note series");
                let amount = row
                    .get(1)
                    .and_then(|value| crate::debt::amount(value, 1000))
                    .context("missing GNL note principal")?;
                payments.push((maturity(&format!(r"{} % Senior Notes, which were [^.]*will mature on [A-Za-z]+ \d+, (20\d{{2}})", regex::escape(&rate)))?, amount));
                principal = principal
                    .checked_add(amount)
                    .context("GNL note principal overflow")?;
            } else if matches!(
                label.as_str(),
                "Less: Deferred financing costs" | "Less: Discount"
            ) {
                adjustments = adjustments
                    .checked_add(
                        row.get(1)
                            .and_then(|value| {
                                crate::debt::amount(value.trim_matches(['(', ')', ' ']), 1000)
                            })
                            .context("missing GNL note adjustment")?,
                    )
                    .context("GNL note adjustment overflow")?;
            }
        }
        anyhow::ensure!(
            seen.len() == 2
                && rows
                    .iter()
                    .find(|row| row
                        .first()
                        .is_some_and(|label| label == "Senior Notes, Net"))
                    .and_then(|row| row.get(1))
                    .and_then(|value| crate::debt::amount(value, 1000))
                    == principal.checked_sub(adjustments),
            "GNL notes do not reconcile"
        );
        return crate::debt::add_payments(debt, &payments);
    }
    crate::debt::tables(
        html,
        as_of,
        &[crate::debt::Rule {
            anchor: "",
            heading: "Total Debt",
            intro: "expected maturity dates and fixed/variable classification",
            ..crate::debt::Rule::new(2, 1000)
        }],
        None,
    )
}
