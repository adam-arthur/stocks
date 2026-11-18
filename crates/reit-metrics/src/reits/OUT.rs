use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "OUT",
        1579877,
        &[
            "outfront.com",
            "www.outfront.com",
            "outfrontmedia.com",
            "www.outfrontmedia.com",
            "investor.outfront.com",
            "investor.outfrontmedia.com",
        ],
        vec![
            (
                Metric::Affo,
                Unit::PerShare,
                r"^affo per weighted average share outstanding: diluted$",
                1,
            ),
            (
                Metric::Affo,
                Unit::Total,
                r"^(?:affo attributable to outfront media inc\.|consolidated affo)$",
                1000000,
            ),
            (
                Metric::Ffo,
                Unit::Total,
                r"^(?:funds from operations|ffo attributable to outfront media inc\.)$",
                1000000,
            ),
        ],
        None,
    ))?;
    company.tables = tables;
    company.sections.push(affo_per_share);
    company.affo_support = crate::AffoSupport::Reported;
    company.debt = Some(debt);
    company.ownership = Some(crate::ownership::Rules::default());
    Ok(company)
}

fn tables(html: &str) -> Vec<crate::table::Table> {
    crate::table::joined_parentheses(html)
        .into_iter()
        .map(|mut table| {
            if let Some(index) = table.rows.iter().take(7).position(|row| {
                row.iter().any(|c| c.text.eq_ignore_ascii_case("Reported"))
                    && row.iter().any(|c| c.text.to_lowercase().contains("basis"))
            }) {
                let reported = table.rows[index]
                    .iter()
                    .filter(|c| c.text.eq_ignore_ascii_case("Reported"))
                    .cloned()
                    .collect::<Vec<_>>();
                for cell in table.rows.iter_mut().skip(index + 1).flatten() {
                    if crate::number(&cell.text).is_some()
                        && !reported.iter().any(|c| c.covers(cell.column))
                    {
                        cell.text.clear();
                    }
                }
            }
            if table
                .rows
                .iter()
                .flatten()
                .any(|cell| cell.text == "AFFO per weighted average share outstanding:")
            {
                for cell in table
                    .rows
                    .iter_mut()
                    .flatten()
                    .filter(|cell| cell.text == "AFFO")
                {
                    cell.text = "Consolidated AFFO".into();
                }
            }
            table
        })
        .collect()
}

fn debt(html: &scraper::Html, as_of: chrono::NaiveDate) -> anyhow::Result<crate::debt::Debt> {
    crate::debt::tables(
        html,
        as_of,
        &[crate::debt::Rule {
            row: Some(("Long-term debt", 0)),
            ..crate::debt::Rule::new(0, 1000000)
        }],
        None,
    )
}

fn affo_per_share(tables: &[crate::table::Table], source: &str) -> Result<Vec<crate::Observation>> {
    let values = crate::affo_per_share(
        company()?,
        tables,
        source,
        crate::companies::Rule {
            metric: Metric::Affo,
            unit: Unit::Total,
            label: regex::Regex::new(r"^diluted$")?,
            scale: Some(1000000),
            section: Some(regex::Regex::new(
                r"(?i)^weighted average shares outstanding:$",
            )?),
        },
    )?;
    Ok(values
        .into_iter()
        .filter(|value| value.start >= chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap())
        .collect())
}
