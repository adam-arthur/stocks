use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "LAND",
        1495240,
        &["www.gladstonefarms.com"],
        vec![
            (
                Metric::Affo,
                Unit::PerShare,
                r"^(?:(?:diluted )?affo per weighted[- ]average (?:common|total(?: common)?) share|affo available to common stockholders(?: and (?:non-controlling op unitholders|op unit ?holders))?\s*[-–—]\s*diluted)$",
                1,
            ),
            (
                Metric::Affo,
                Unit::Total,
                r"^affo available to common stockholders(?: and (?:non-controlling op unitholders|op unit ?holders))?$",
                0,
            ),
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^(?:(?:diluted )?ffo per weighted[- ]average (?:common|total(?: common)?) share|ffo available to common stockholders(?: and (?:non-controlling op unitholders|op unit ?holders))?\s*[-–—]\s*diluted)$",
                1,
            ),
            (
                Metric::Ffo,
                Unit::Total,
                r"^ffo available to common stockholders(?: and (?:non-controlling op unitholders|op unit ?holders))?$",
                0,
            ),
        ],
        None,
    ))?;
    company.affo_support = crate::AffoSupport::Reported;
    company.tables = |html| {
        let annual = crate::table::normalize(
            &scraper::Html::parse_document(html)
                .root_element()
                .text()
                .collect::<Vec<_>>()
                .join(" "),
        )
        .to_lowercase()
        .contains("fourth quarter and year ended december 31");
        crate::table::joined_parentheses(html)
            .into_iter()
            .map(|mut table| {
                if annual
                    && table.rows.iter().take(4).flatten().any(|c| {
                        c.text
                            .to_lowercase()
                            .contains("for and as of the quarters ended")
                    })
                {
                    for cell in table.rows.iter_mut().take(4).flatten() {
                        if cell.text.len() == 4
                            && cell.text.starts_with("20")
                            && cell.text.parse::<u32>().is_ok()
                        {
                            cell.text = format!("Three months ended December 31, {}", cell.text);
                        }
                    }
                }
                table.amount_scale = Some(table.amount_scale.unwrap_or(1));
                table
            })
            .collect()
    };
    company.debt = Some(debt);
    Ok(company)
}

fn debt(html: &scraper::Html, as_of: chrono::NaiveDate) -> anyhow::Result<crate::debt::Debt> {
    let debt = crate::debt::tables(html, as_of, &[crate::debt::Rule::new(0, 1000)], None)?;
    let payments = crate::debt::instrument_payments(
        &crate::debt::rows(html, as_of, "Undrawn Commitment")?,
        (r"^MetLife Lines of Credit$", 3, 2),
    )?;
    anyhow::ensure!(
        crate::debt::scalar(html, "us-gaap:LineOfCredit", as_of)?
            == Some(i64::try_from(
                payments.iter().map(|(_, value)| value).sum::<u64>()
            )?),
        "LAND credit facility balance does not reconcile"
    );
    crate::debt::add_payments(debt, &payments)
}
