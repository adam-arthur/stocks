use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "REXR",
        1571283,
        &[
            "www.rexfordindustrial.com",
            "rexfordindustrial.com",
            "ir.rexfordindustrial.com",
        ],
        vec![
            (
                Metric::Ffo,
                Unit::PerShare,
                r"^(?:company share of ffo per common share\s*[-‐–—]\s*diluted|ffo per (?:common )?share\s*[-‐–—]\s*basic and diluted)$",
                1,
            ),
            (Metric::Ffo, Unit::Total, r"^company share of ffo$", 1000),
        ],
        None,
    ))?;
    company.tables = tables;
    Ok(company)
}

fn tables(html: &str) -> Vec<crate::table::Table> {
    crate::table::joined_parentheses(html)
        .into_iter()
        .map(|mut table| {
            let years = table.rows.iter().take(8).find_map(|row| {
                let years = row
                    .iter()
                    .filter(|cell| {
                        cell.text
                            .parse::<u32>()
                            .is_ok_and(|year| (2000..2100).contains(&year))
                    })
                    .cloned()
                    .collect::<Vec<_>>();
                (years.len() == 4).then_some(years)
            });
            if let Some(years) = years {
                for row in table.rows.iter_mut().take(8) {
                    let mut headers = row
                        .iter_mut()
                        .filter(|cell| crate::duration(&cell.text).is_some())
                        .collect::<Vec<_>>();
                    if headers.len() == 2 {
                        for (header, years) in headers.iter_mut().zip(years.chunks_exact(2)) {
                            header.column = years[0].column;
                            header.width = years[1].column + years[1].width - header.column;
                        }
                    }
                }
            }
            table
        })
        .collect()
}
