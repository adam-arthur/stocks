use std::collections::{BTreeMap, HashMap};

use scraper::{ElementRef, Html, Selector};

#[derive(Clone, Debug)]
pub struct Cell {
    pub text: String,
    pub column: usize,
    pub width: usize,
}

impl Cell {
    pub fn covers(&self, column: usize) -> bool {
        (self.column..self.column + self.width).contains(&column)
    }
}

#[derive(Debug)]
pub struct Table {
    pub rows: Vec<Vec<Cell>>,
    pub amount_scale: Option<u64>,
}

impl Table {
    pub fn is_forecast(&self) -> bool {
        self.rows.iter().any(|row| {
            row.iter().any(|cell| {
                ["Low", "Low End", "Low Case"]
                    .iter()
                    .any(|label| cell.text.eq_ignore_ascii_case(label))
            }) && row.iter().any(|cell| {
                ["High", "High End", "High Case"]
                    .iter()
                    .any(|label| cell.text.eq_ignore_ascii_case(label))
            })
        }) || self.rows.iter().take(6).flatten().any(|cell| {
            let text = cell.text.to_lowercase();
            text.contains("guidance") || text.contains("outlook")
        })
    }
}

pub fn normalize(text: &str) -> String {
    text.replace(['\u{200b}', '\u{feff}', '\u{ad}'], "")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn tables(html: &str) -> Vec<Table> {
    let html = Html::parse_document(html);
    let selector = Selector::parse("table").unwrap();
    let rows = Selector::parse("tr").unwrap();
    let local_units = regex::Regex::new(r"(?i)^(?:\(\s*(?:(?:dollars?|amounts|\$)\s+)?|\b(?:dollars?|amounts|\$)\s+)in\s+(thousands|millions)\b").unwrap();
    let units =
        regex::Regex::new(r"(?i)(?:\$|dollars?|amounts)\s+in\s+(thousands|millions)|\(\s*in\s+(thousands|millions)[^()]{0,90}\)\s*\(unaudited\)").unwrap();
    let mut text = String::new();
    let mut offsets = HashMap::new();
    for node in html.root_element().descendants() {
        if node
            .value()
            .as_element()
            .is_some_and(|element| element.name() == "table")
        {
            offsets.insert(node.id(), text.len());
        }
        if let Some(value) = node.value().as_text() {
            text.push_str(&normalize(value));
            text.push(' ');
        }
    }
    let scales: Vec<_> = units
        .captures_iter(&text)
        .map(|matched| {
            (
                matched.get(0).unwrap().end(),
                if matched
                    .get(1)
                    .or_else(|| matched.get(2))
                    .unwrap()
                    .as_str()
                    .eq_ignore_ascii_case("millions")
                {
                    1_000_000
                } else {
                    1000
                },
            )
        })
        .collect();
    html.select(&selector)
        .filter(|table| table.select(&selector).next().is_none())
        .map(|table| {
            let mut occupied = BTreeMap::<usize, (usize, Cell)>::new();
            Table {
                amount_scale: table
                    .select(&rows)
                    .filter(|row| row.text().any(|text| !text.trim().is_empty()))
                    .take(8)
                    .flat_map(|row| row.children().filter_map(ElementRef::wrap))
                    .find_map(|cell| {
                        let text = normalize(&cell.text().collect::<Vec<_>>().join(" "));
                        local_units.captures(&text).map(|found| {
                            if found[1].eq_ignore_ascii_case("millions") {
                                1_000_000
                            } else {
                                1000
                            }
                        })
                    })
                    .or_else(|| {
                        scales[..scales.partition_point(|(end, _)| *end <= offsets[&table.id()])]
                            .last()
                            .map(|(_, scale)| *scale)
                    }),
                rows: table
                    .select(&rows)
                    .enumerate()
                    .map(|(row_number, row)| {
                        occupied.retain(|_, (until, _)| *until > row_number);
                        let mut result: Vec<_> =
                            occupied.values().map(|(_, cell)| cell.clone()).collect();
                        let mut column = 0;
                        for cell in row
                            .children()
                            .filter_map(ElementRef::wrap)
                            .filter(|cell| matches!(cell.value().name(), "td" | "th"))
                        {
                            while result.iter().any(|cell| cell.covers(column)) {
                                column += 1;
                            }
                            let parsed = Cell {
                                text: normalize(&cell.text().collect::<Vec<_>>().join(" ")),
                                column,
                                width: cell
                                    .value()
                                    .attr("colspan")
                                    .and_then(|v| v.parse().ok())
                                    .unwrap_or(1)
                                    .clamp(1, 500),
                            };
                            let height = cell
                                .value()
                                .attr("rowspan")
                                .and_then(|v| v.parse::<usize>().ok())
                                .unwrap_or(1)
                                .clamp(1, 500);
                            if height > 1 {
                                occupied.insert(column, (row_number + height, parsed.clone()));
                            }
                            column += parsed.width;
                            result.push(parsed);
                        }
                        result.sort_by_key(|cell| cell.column);
                        result
                    })
                    .collect(),
            }
        })
        .map(|mut table| {
            if table.rows.iter().any(|row| {
                row.iter()
                    .any(|cell| cell.text.eq_ignore_ascii_case("As Previously Reported"))
                    && row
                        .iter()
                        .any(|cell| cell.text.eq_ignore_ascii_case("Adjustments"))
            }) {
                table.rows.clear();
            }
            table
        })
        .collect()
}

pub(crate) fn joined_parentheses(html: &str) -> Vec<Table> {
    let mut tables = tables(html);
    for row in tables.iter_mut().flat_map(|table| &mut table.rows) {
        for index in 1..row.len() {
            if row[index].text == ")"
                && row[index - 1].text.starts_with('(')
                && !row[index - 1].text.ends_with(')')
            {
                row[index - 1].text.push(')');
                row[index].text.clear();
            }
        }
    }
    tables
}

pub(crate) fn align_year_pairs(tables: &mut [Table]) {
    let year = regex::Regex::new(r"^20\d{2}(?: Actual)?$").unwrap();
    for table in tables {
        for i in 1..table.rows.len() {
            let years: Vec<_> = table.rows[i]
                .iter()
                .filter(|cell| year.is_match(&cell.text))
                .cloned()
                .collect();
            if years.len() != 4 || years[0].text != years[2].text || years[1].text != years[3].text
            {
                continue;
            }
            let mut headings: Vec<_> = table.rows[i - 1]
                .iter_mut()
                .filter(|cell| crate::duration(&cell.text).is_some())
                .collect();
            if headings.len() == 2
                && crate::duration(&headings[0].text) == Some(3)
                && crate::duration(&headings[1].text) != Some(3)
            {
                for (heading, pair) in headings.iter_mut().zip(years.chunks_exact(2)) {
                    heading.column = pair[0].column;
                    heading.width = pair[1].column + pair[1].width - pair[0].column;
                }
            }
        }
    }
}
