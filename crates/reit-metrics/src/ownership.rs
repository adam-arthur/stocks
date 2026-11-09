use std::collections::{BTreeMap, BTreeSet};

use anyhow::Result;
use chrono::NaiveDate;
use regex::Regex;
use scraper::{ElementRef, Html, Selector};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::table::normalize;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Count {
    pub value: f64,
    pub source: String,
    pub evidence: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filed: Option<NaiveDate>,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct Ownership {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shares_outstanding: Option<Count>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outside_op_units: Option<Count>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outside_op_unit_share_equivalents: Option<Count>,
}

pub(crate) type FactRule = (&'static str, &'static [&'static str]);
pub(crate) type Extractor =
    fn(&str, &str, Option<NaiveDate>, &mut BTreeMap<NaiveDate, Ownership>) -> Result<()>;

#[derive(Default)]
pub(crate) struct Rules {
    pub units: &'static [FactRule],
    pub classes: &'static [FactRule],
    pub exchange_pattern: Option<&'static str>,
    pub exchange_source: Option<&'static str>,
    pub prose: Option<Extractor>,
    pub tables: Option<Extractor>,
}

pub fn supports(symbol: &str) -> bool {
    crate::companies::company(symbol).is_ok_and(|company| company.ownership.is_some())
}

pub(crate) fn date(value: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(&value.replace(',', ""), "%B %d %Y").ok()
}

pub(crate) fn report_date(text: &str) -> Option<NaiveDate> {
    let matched = Regex::new(r"(?i)For the (?:q\s*uarter\s*ly period|quarter|fiscal year|year) ended:?\s+([A-Za-z]+)\s+(\d(?:\s*\d)?)\s*,\s*(20\s*\d\s*\d)").ok()?.captures(&text[..text.char_indices().nth(2000).map_or(text.len(), |(index, _)| index)])?;
    date(&format!(
        "{} {}, {}",
        &matched[1],
        matched[2].replace(' ', ""),
        matched[3].replace(' ', "")
    ))
}

pub fn common_shares(facts: &Value, end: NaiveDate, as_of: NaiveDate) -> Option<Count> {
    for (namespace, tag) in [
        ("us-gaap", "CommonStockSharesOutstanding"),
        ("dei", "EntityCommonStockSharesOutstanding"),
    ] {
        let observations = facts["facts"][namespace][tag]["units"]["shares"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|value| {
                value["end"].as_str() == Some(end.to_string().as_str())
                    && value.get("start").is_none()
                    && value["filed"]
                        .as_str()
                        .is_some_and(|date| date <= as_of.to_string().as_str())
                    && matches!(
                        value["form"].as_str(),
                        Some("10-Q" | "10-Q/A" | "10-K" | "10-K/A")
                    )
                    && value["val"]
                        .as_f64()
                        .is_some_and(|value| value.is_finite() && value > 0.0)
            })
            .collect::<Vec<_>>();
        let Some(latest) = observations
            .iter()
            .map(|value| value["filed"].as_str().unwrap())
            .max()
        else {
            continue;
        };
        let selected = observations.iter().find(|value| value["filed"] == latest)?;
        if observations
            .iter()
            .any(|value| value["filed"] == latest && value["val"] != selected["val"])
        {
            return None;
        }
        return Some(Count {
            value: selected["val"].as_f64()?,
            source: format!(
                "https://www.sec.gov/Archives/edgar/data/{}/{}/{}-index.html",
                sec_client::company_cik(facts)?,
                selected["accn"].as_str()?.replace('-', ""),
                selected["accn"].as_str()?
            ),
            evidence: format!("{namespace}:{tag}; instant {end}"),
            filed: latest.parse().ok(),
        });
    }
    None
}

struct Fact {
    name: String,
    members: BTreeSet<String>,
    end: NaiveDate,
    value: f64,
    issued_and_outstanding: bool,
}

fn text(element: ElementRef<'_>) -> String {
    normalize(&element.text().collect::<Vec<_>>().join(" "))
}

fn inline_facts(html: &Html, cik: u64) -> Vec<Fact> {
    let elements = html
        .select(&Selector::parse("*").unwrap())
        .collect::<Vec<_>>();
    let contexts = elements
        .iter()
        .filter(|element| element.value().name() == "xbrli:context")
        .filter_map(|element| {
            let descendants = element
                .descendants()
                .filter_map(ElementRef::wrap)
                .collect::<Vec<_>>();
            if descendants
                .iter()
                .find(|element| element.value().name() == "xbrli:identifier")
                .and_then(|element| text(*element).parse::<u64>().ok())
                != Some(cik)
            {
                return None;
            }
            if descendants.iter().any(|element| {
                matches!(
                    element.value().name(),
                    "xbrli:startdate" | "xbrldi:typedmember"
                )
            }) {
                return None;
            }
            Some((
                element.value().attr("id")?.to_owned(),
                (
                    descendants
                        .iter()
                        .find(|element| element.value().name() == "xbrli:instant")
                        .and_then(|element| text(*element).parse::<NaiveDate>().ok())?,
                    descendants
                        .iter()
                        .filter(|element| element.value().name() == "xbrldi:explicitmember")
                        .map(|element| text(*element).to_lowercase())
                        .collect::<BTreeSet<_>>(),
                ),
            ))
        })
        .collect::<BTreeMap<_, _>>();
    let units = elements
        .iter()
        .filter(|element| element.value().name() == "xbrli:unit")
        .filter(|element| text(**element).eq_ignore_ascii_case("xbrli:shares"))
        .filter_map(|element| element.value().attr("id"))
        .collect::<BTreeSet<_>>();
    elements
        .iter()
        .filter(|element| element.value().name() == "ix:nonfraction")
        .filter_map(|element| {
            if element.value().attr("xsi:nil") == Some("true")
                || !units.contains(element.value().attr("unitref")?)
            {
                return None;
            }
            let (end, members) = contexts.get(element.value().attr("contextref")?)?;
            let value = match text(*element).as_str() {
                "—" | "–" | "-" => 0.0,
                value => value.replace([',', ' '], "").parse::<f64>().ok()?,
            } * 10_f64.powi(element.value().attr("scale").unwrap_or("0").parse().ok()?);
            if !value.is_finite() || value < 0.0 || element.value().attr("sign") == Some("-") {
                return None;
            }
            Some(Fact {
                name: element.value().attr("name")?.to_lowercase(),
                issued_and_outstanding: element
                    .ancestors()
                    .filter_map(ElementRef::wrap)
                    .find(|element| element.value().name() == "td")
                    .is_some_and(|element| {
                        text(element)
                            .to_lowercase()
                            .contains("issued and outstanding")
                    }),
                members: members.clone(),
                end: *end,
                value,
            })
        })
        .collect()
}

pub fn extract(
    symbol: &str,
    html: &str,
    source: &str,
    filed: Option<NaiveDate>,
) -> Result<BTreeMap<NaiveDate, Ownership>> {
    let company = crate::companies::company(symbol)?;
    crate::source::validate_url(&company, source)?;
    let rules = company.ownership.unwrap_or_default();
    let document = Html::parse_document(html);
    let facts = inline_facts(&document, company.cik);
    let mut result = BTreeMap::<NaiveDate, Ownership>::new();
    for end in facts.iter().map(|fact| fact.end).collect::<BTreeSet<_>>() {
        if filed.is_some_and(|filed| end > filed) {
            continue;
        }
        for (name, members) in [
            ("us-gaap:commonstocksharesoutstanding", &[][..]),
            (
                "us-gaap:sharesoutstanding",
                &["us-gaap:commonstockmember"][..],
            ),
            ("dei:entitycommonstocksharesoutstanding", &[][..]),
        ] {
            let matches = facts
                .iter()
                .filter(|fact| {
                    fact.end == end
                        && fact.name == name
                        && fact.members
                            == members
                                .iter()
                                .map(|member| member.to_string())
                                .collect::<BTreeSet<_>>()
                })
                .collect::<Vec<_>>();
            if let Some(first) = matches.first().filter(|first| {
                first.value > 0.0 && matches.iter().all(|other| other.value == first.value)
            }) {
                result.entry(end).or_default().shares_outstanding = Some(Count {
                    value: first.value,
                    source: source.into(),
                    evidence: format!("{name}; instant {end}"),
                    filed,
                });
                break;
            }
        }
        let classes = rules.classes;
        if !classes.is_empty() {
            let values = classes
                .iter()
                .map(|(name, members)| {
                    let matches = facts
                        .iter()
                        .filter(|fact| {
                            fact.end == end
                                && fact.name == *name
                                && fact.members
                                    == members
                                        .iter()
                                        .map(|member| member.to_string())
                                        .collect::<BTreeSet<_>>()
                                && (*name != "us-gaap:commonstocksharesissued"
                                    || fact.issued_and_outstanding)
                        })
                        .collect::<Vec<_>>();
                    matches
                        .first()
                        .filter(|first| matches.iter().all(|other| other.value == first.value))
                        .map(|first| first.value)
                })
                .collect::<Option<Vec<_>>>();
            if let Some(values) = values {
                result.entry(end).or_default().shares_outstanding = Some(Count {
                    value: values.iter().sum(),
                    source: source.into(),
                    filed,
                    evidence: format!(
                        "{}; instant {end}",
                        classes
                            .iter()
                            .map(|(name, members)| format!("{name} [{}]", members.join(", ")))
                            .collect::<Vec<_>>()
                            .join(" + ")
                    ),
                });
            }
        }
        let mut units = Vec::new();
        for (name, members) in rules.units {
            let matches = facts
                .iter()
                .filter(|fact| {
                    fact.end == end
                        && fact.name.eq_ignore_ascii_case(name)
                        && fact.members
                            == members
                                .iter()
                                .map(|member| member.to_lowercase())
                                .collect::<BTreeSet<_>>()
                })
                .collect::<Vec<_>>();
            if let Some(first) = matches
                .first()
                .filter(|first| matches.iter().all(|other| other.value == first.value))
            {
                units.push(Count {
                    value: first.value,
                    source: source.into(),
                    evidence: format!("{} [{}]; instant {end}", name, members.join(", ")),
                    filed,
                });
            }
        }
        if !units.is_empty() && units.len() == rules.units.len() {
            let count = Count {
                value: units.iter().map(|count| count.value).sum(),
                evidence: units
                    .iter()
                    .map(|count| count.evidence.as_str())
                    .collect::<Vec<_>>()
                    .join(" + "),
                ..units[0].clone()
            };
            result.entry(end).or_default().outside_op_units = Some(count);
        }
    }
    let visible = normalize(
        &document
            .root_element()
            .descendants()
            .filter_map(|node| {
                if node
                    .ancestors()
                    .filter_map(ElementRef::wrap)
                    .any(|element| {
                        matches!(element.value().name(), "ix:header" | "script" | "style")
                    })
                {
                    None
                } else {
                    node.value().as_text().map(|text| text.to_string())
                }
            })
            .collect::<Vec<_>>()
            .join(" "),
    );
    if let Some(prose) = rules.prose {
        prose(&visible, source, filed, &mut result)?;
    }
    if let Some(tables) = rules.tables {
        tables(html, source, filed, &mut result)?;
    }
    if let Some(evidence) = rules
        .exchange_pattern
        .map(Regex::new)
        .transpose()?
        .and_then(|pattern| {
            pattern
                .find(&visible)
                .map(|matched| matched.as_str().to_owned())
        })
        .or_else(|| {
            rules.exchange_source.map(|source| {
                format!("Researched one-for-one common-unit exchange terms: {source}")
            })
        })
    {
        for ownership in result.values_mut() {
            if let Some(count) = &ownership.outside_op_units {
                ownership.outside_op_unit_share_equivalents = Some(Count {
                    evidence: format!("{}; {}", count.evidence, evidence),
                    ..count.clone()
                });
            }
        }
    }
    result.retain(|end, _| filed.is_none_or(|filed| *end <= filed));
    Ok(result)
}

pub fn merge(target: &mut Ownership, incoming: Ownership) {
    if incoming.outside_op_unit_share_equivalents.is_none()
        && incoming.outside_op_units.as_ref().is_some_and(|incoming| {
            target
                .outside_op_units
                .as_ref()
                .is_some_and(|old| incoming.filed >= old.filed && incoming.value != old.value)
        })
    {
        target.outside_op_unit_share_equivalents = None;
    }
    for (target, incoming) in [
        (&mut target.shares_outstanding, incoming.shares_outstanding),
        (&mut target.outside_op_units, incoming.outside_op_units),
        (
            &mut target.outside_op_unit_share_equivalents,
            incoming.outside_op_unit_share_equivalents,
        ),
    ] {
        if let Some(incoming) = incoming
            && target
                .as_ref()
                .is_none_or(|old| incoming.filed >= old.filed)
        {
            *target = Some(incoming);
        }
    }
}
