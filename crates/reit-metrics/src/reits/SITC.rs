use crate::companies::{Company, build};
use crate::{Metric, Unit};
use anyhow::Result;

pub fn company() -> Result<Company> {
    let mut company = build((
        "SITC",
        894315,
        &[
            "www.sitecenters.com",
            "ir.sitecenters.com",
            "images.sitecenters.com",
        ],
        vec![(
            Metric::Ffo,
            Unit::PerShare,
            r"^ffo per share\s*[-–—]\s*(?:basic & )?diluted$",
            1,
        )],
        None,
    ))?;
    company.tables = tables;
    Ok(company)
}

fn tables(html: &str) -> Vec<crate::table::Table> {
    let period = regex::Regex::new(r"^([1-4])Q(\d{2})$|^(3|6|9|12)M(\d{2})$|^FY(\d{2})$").unwrap();
    crate::table::joined_parentheses(html)
        .into_iter()
        .map(|mut table| {
            for cell in table.rows.iter_mut().flatten() {
                if let Some(period) = period.captures(&cell.text) {
                    cell.text = if let Some(quarter) = period.get(1) {
                        format!("Q{} 20{}", quarter.as_str(), &period[2])
                    } else if let Some(months) = period.get(3) {
                        format!(
                            "{} months ended {} 20{}",
                            match months.as_str() {
                                "3" => "Three",
                                "6" => "Six",
                                "9" => "Nine",
                                _ => "Twelve",
                            },
                            match months.as_str() {
                                "3" => "March 31",
                                "6" => "June 30",
                                "9" => "September 30",
                                _ => "December 31",
                            },
                            &period[4]
                        )
                    } else {
                        format!("Year ended December 31 20{}", &period[5])
                    };
                }
            }
            table
        })
        .collect()
}
