use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "BXP",
        1037540,
        &["www.bxp.com", "ir.bxp.com", "ir.bostonproperties.com"],
        vec![
            (
                Metric::Affo,
                Unit::Total,
                r"^funds available for distribution to common shareholders and common unitholders(?: fad)?(?:\s+\d+)?$",
                1000,
            ),
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^(?:diluted ffo per share|ffo per share(?: -)? diluted)(?:\s+\d+)?$",
                1,
            ),
            (
                Metric::Ffo,
                Unit::Total,
                r"^(?:ffo|funds from operations) attributable to (?:boston properties|bxp), inc\.(?: common shareholders)?(?:\s+\d+)?$",
                1000,
            ),
        ],
        None,
    ))?;
    company.tables = tables;
    company.sections.push(affo_per_share);
    company.affo_support = crate::AffoSupport::Reported;
    company.debt = Some(debt);
    Ok(company)
}

fn debt(html: &scraper::Html, as_of: chrono::NaiveDate) -> anyhow::Result<crate::debt::Debt> {
    use anyhow::Context;
    use chrono::Datelike;
    // Mortgage balances omit scheduled amortization; average commercial-paper tenor cannot establish calendar-year payments.
    let mortgage = crate::debt::tables(html, as_of, &[crate::debt::Rule::new(0, 1000)], None)
        .context("BXP needs mortgage amortization by year and dated commercial-paper repayments")?;
    let mut payments: Vec<_> = mortgage.maturities.into_iter().collect();
    payments.push((as_of.year() + 6, mortgage.thereafter));
    let mut adjustments = 0;
    let mut reported = None;
    for table in html
        .select(&scraper::Selector::parse("table").unwrap())
        .filter(|table| {
            let text = crate::table::normalize(&table.text().collect::<Vec<_>>().join(" "));
            text.contains("Stated")
                && text.contains("GAAP")
                && text.contains("Maturity Date")
                && text.contains(&as_of.format("%m/%d/%Y").to_string())
        })
    {
        for row in table.select(&scraper::Selector::parse("tr").unwrap()) {
            let cells: Vec<_> = row
                .select(&scraper::Selector::parse("td, th").unwrap())
                .map(|cell| crate::table::normalize(&cell.text().collect::<String>()))
                .filter(|cell| !cell.is_empty() && cell != "$")
                .collect();
            let Some(label) = cells.first() else { continue };
            if label.starts_with("Less:") {
                adjustments += crate::debt::amount(
                    cells.get(1).context("missing BXP financing adjustment")?,
                    1000,
                )
                .context("invalid BXP financing adjustment")?;
            }
            if label == "Total Consolidated Debt" {
                reported =
                    crate::debt::amount(cells.get(1).context("missing BXP total debt")?, 1000);
            }
            if !label.contains("Unsecured") {
                continue;
            }
            if label.starts_with("Unsecured Commercial Paper") {
                anyhow::ensure!(
                    as_of.month() == 12
                        && crate::table::normalize(
                            &html.root_element().text().collect::<Vec<_>>().join(" ")
                        )
                        .contains("varying maturities of up to one year"),
                    "BXP commercial paper needs calendar-year allocation"
                );
                let position = cells
                    .iter()
                    .position(|cell| cell == "Various")
                    .context("missing BXP commercial paper maturity")?;
                payments.push((
                    as_of.year() + 1,
                    crate::debt::amount(
                        cells
                            .get(position + 1)
                            .context("missing BXP commercial paper balance")?,
                        1000,
                    )
                    .context("invalid BXP commercial paper balance")?,
                ));
            } else if let Some((position, date)) =
                cells.iter().enumerate().find_map(|(index, cell)| {
                    chrono::NaiveDate::parse_from_str(cell, "%B %e, %Y")
                        .ok()
                        .map(|date| (index, date))
                })
            {
                let balance = cells
                    .get(position + 1)
                    .context("missing BXP unsecured principal")?;
                if balance != "N/A" {
                    payments.push((
                        date.year(),
                        crate::debt::amount(balance, 1000)
                            .context("invalid BXP unsecured principal")?,
                    ));
                }
            }
        }
    }
    let debt = crate::debt::payments(as_of, &payments)?;
    anyhow::ensure!(
        debt.total_debt_principal
            == reported.context("missing BXP consolidated total")? + adjustments,
        "BXP principal does not reconcile to consolidated debt"
    );
    Ok(debt)
}

fn affo_per_share(tables: &[crate::table::Table], source: &str) -> Result<Vec<crate::Observation>> {
    let values = crate::affo_per_share(
        company()?,
        tables,
        source,
        crate::companies::Rule {
            metric: Metric::Affo,
            unit: Unit::Total,
            label: regex::Regex::new(r"^shares/units for diluted ffo$")?,
            scale: Some(1000),
            section: None,
        },
    )?;
    let mut company = company()?;
    company.rules = vec![crate::companies::Rule {
        metric: Metric::Affo,
        unit: Unit::Total,
        label: regex::Regex::new(r"^effect of dilutive securities - stock-based compensation$")?,
        scale: Some(1000),
        section: None,
    }];
    let adjustments = crate::reported_sections(&company, tables, source)?.0;
    Ok(values
        .into_iter()
        .filter(|value| {
            adjustments.iter().any(|(_, adjustment)| {
                adjustment.start == value.start && adjustment.end == value.end
            }) && adjustments.iter().all(|(_, adjustment)| {
                adjustment.start != value.start
                    || adjustment.end != value.end
                    || adjustment.value == 0.0
            })
        })
        .collect())
}

fn tables(html: &str) -> Vec<crate::table::Table> {
    let mut tables = crate::table::joined_parentheses(html);
    for row in tables.iter_mut().flat_map(|table| &mut table.rows) {
        if row.first().is_some_and(|cell| {
            cell.text
                .eq_ignore_ascii_case("Effect of dilutive securities - stock-based compensation")
        }) {
            for cell in row.iter_mut().skip(1) {
                if matches!(cell.text.as_str(), "—" | "–" | "-") {
                    cell.text = "0".into();
                }
            }
        }
    }
    tables
}
