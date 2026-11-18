use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "KRC",
        1025996,
        &["www.kilroyrealty.com", "investors.kilroyrealty.com"],
        vec![
            (
                Metric::Affo,
                Unit::Total,
                r"^funds available for distribution$",
                1000,
            ),
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^ffo per common share/unit\s*[-–—]\s*diluted$",
                1,
            ),
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^funds from operations per common share(?:/unit)?\s*[-–—]\s*diluted$",
                1,
            ),
            (Metric::Ffo, Unit::Total, r"^funds from operations$", 1000),
        ],
        None,
    ))?;
    company.tables = |html| {
        let mut tables = crate::table::tables(html);
        for table in &mut tables {
            for i in 1..table.rows.len() {
                let headings = table.rows[i - 1]
                    .iter()
                    .filter(|c| crate::duration(&c.text).is_some())
                    .map(|c| c.text.clone())
                    .collect::<Vec<_>>();
                if headings.first().and_then(|h| crate::duration(h)) != Some(3) {
                    continue;
                }
                let dates = table.rows[i]
                    .iter_mut()
                    .filter_map(|c| {
                        chrono::NaiveDate::parse_from_str(&c.text, "%m/%d/%Y")
                            .ok()
                            .map(|d| (c, d))
                    })
                    .collect::<Vec<_>>();
                if !(dates.len() == 5 && headings.len() == 1
                    || dates.len() == 7
                        && headings.len() == 2
                        && dates[0].1 == dates[5].1
                        && dates[4].1 == dates[6].1)
                    || !dates[..5].windows(2).all(|w| w[0].1 > w[1].1)
                {
                    continue;
                }
                for (index, (cell, date)) in dates.into_iter().enumerate() {
                    cell.text = format!(
                        "{} {}",
                        headings[usize::from(index >= 5)],
                        date.format("%B %d, %Y")
                    );
                }
            }
        }
        tables
    };
    company.affo_support = crate::AffoSupport::Reported;
    company.sections.push(affo_per_share);
    company.debt = Some(debt);
    Ok(company)
}

fn affo_per_share(tables: &[crate::table::Table], source: &str) -> Result<Vec<crate::Observation>> {
    crate::affo_per_share(
        company()?,
        tables,
        source,
        crate::companies::Rule {
            metric: Metric::Affo,
            unit: Unit::Total,
            label: regex::Regex::new(
                r"^weighted average common shares/units outstanding\s*[-–—]\s*diluted$",
            )?,
            scale: Some(1000),
            section: None,
        },
    )
}

fn debt(html: &scraper::Html, as_of: chrono::NaiveDate) -> anyhow::Result<crate::debt::Debt> {
    crate::debt::tables(html, as_of, &[crate::debt::Rule::new(0, 1000)], None)
}
