use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;
pub fn company() -> Result<Company> {
    let mut company = build((
        "LTC",
        887905,
        &["ir.ltcreit.com"],
        vec![
            (
                Metric::Affo,
                Unit::Total,
                r"^(?:normalized )?adjusted ffo(?: \(affo\))?$",
                1000,
            ),
            (
                Metric::Affo,
                Unit::Total,
                r#"^funds available for distribution(?: \("?fad"?\))?$"#,
                1000,
            ),
            (Metric::Affo, Unit::PerShare, r"^diluted fad per share$", 1),
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^(?:nareit diluted ffo per common share|(?:nareit diluted|diluted(?: nareit)?) ffo attributable to common stockholders per share)$",
                1,
            ),
            (
                Metric::Ffo,
                Unit::Total,
                r"^(?:nareit )?(?:funds from operations|ffo) attributable to common stockholders$",
                1000,
            ),
        ],
        None,
    ))?;
    company.affo_support = crate::AffoSupport::Reported;
    company.sections.push(affo_per_share);
    company.tables = crate::table::joined_parentheses;
    company.debt = Some(debt);
    company.ownership = Some(crate::ownership::Rules::default());
    Ok(company)
}

fn affo_per_share(tables: &[crate::table::Table], source: &str) -> Result<Vec<crate::Observation>> {
    let mut observations = Vec::<crate::Observation>::new();
    for measure in ["(?:normalized )?affo", "fad"] {
        let mut company = company()?;
        company.rules = vec![crate::companies::Rule {
            metric: Metric::Affo,
            unit: Unit::Total,
            label: regex::Regex::new(&format!("^diluted {measure}$"))?,
            scale: Some(1000),
            section: None,
        }];
        for value in crate::affo_per_share(
            company,
            tables,
            source,
            crate::companies::Rule {
                metric: Metric::Affo,
                unit: Unit::Total,
                label: regex::Regex::new(&format!(
                    "^weighted average shares used to calculate diluted {measure} per share$"
                ))?,
                scale: Some(1000),
                section: None,
            },
        )? {
            if !observations
                .iter()
                .any(|existing| existing.start == value.start && existing.end == value.end)
            {
                observations.push(value);
            }
        }
    }
    Ok(observations)
}

fn debt(html: &scraper::Html, as_of: chrono::NaiveDate) -> anyhow::Result<crate::debt::Debt> {
    crate::debt::tables(
        html,
        as_of,
        &[crate::debt::Rule {
            row: Some(("$", 1)),
            ..crate::debt::Rule::new(0, 1000)
        }],
        None,
    )
}
