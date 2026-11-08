use std::collections::BTreeMap;

use anyhow::Result;
use chrono::{Datelike, NaiveDate};
use regex::Regex;

use super::{Count, Ownership};

type Rule = (&'static str, &'static [&'static str]);

pub(super) fn units(symbol: &str) -> &'static [Rule] {
    match symbol {
        "BHM" => &[(
            "us-gaap:PartnersCapitalAccountUnits",
            &["bhm:OpUnitHoldersMember"],
        )],
        "DRH" => &[(
            "us-gaap:UnitsOfPartnershipInterestAmount",
            &["drh:UnaffiliatedThirdPartiesMember"],
        )],
        "FVR" => &[(
            "us-gaap:CommonUnitOutstanding",
            &["fvr:OwnershipInterestMember"],
        )],
        "HIW" => &[(
            "hiw:CommonUnitsOfPartnershipNotOwned",
            &["srt:SubsidiariesMember"],
        )],
        "RLJ" => &[(
            "us-gaap:PartnersCapitalAccountUnits",
            &["us-gaap:LimitedPartnerMember"],
        )],
        "STAG" => &[(
            "us-gaap:LimitedPartnersCapitalAccountUnitsOutstanding",
            &["stag:OtherNoncontrollingCommonUnitsMember"],
        )],
        "STRW" => &[(
            "us-gaap:LimitedPartnersCapitalAccountUnitsOutstanding",
            &["STRW:OperatingPartnershipAgreementMember"],
        )],
        "VNO" => &[(
            "us-gaap:TemporaryEquitySharesOutstanding",
            &["us-gaap:CapitalUnitClassAMember"],
        )],
        "LINE" => &[
            (
                "us-gaap:UnitsOfPartnershipInterestAmount",
                &[
                    "line:LineageOPLLCMember",
                    "line:PartnershipCommonUnitsMember",
                ],
            ),
            (
                "us-gaap:UnitsOfPartnershipInterestAmount",
                &[
                    "line:LineageOPLLCMember",
                    "line:LegacyOperatingPartnershipUnitClassAAndClassBMember",
                ],
            ),
        ],
        _ => &[],
    }
}

pub(super) fn exchange_evidence(symbol: &str, text: &str) -> Result<Option<String>> {
    let pattern = match symbol {
        "BFS" => {
            r"(?i)units are convertible into shares of.{0,180}Saul Centers. common stock.{0,70}one\s*-for-one basis"
        }
        "BHM" => {
            r"(?i)OP Units(?:, which)? are exchangeable for Class A common stock on a one\s*-for-one basis"
        }
        "DRH" => {
            r"(?i)common OP units for cash per unit.{0,160}common stock on a one\s*-for-one basis"
        }
        "FVR" => {
            r"(?i)OP Units.{0,160}redeemable into the Company.s common stock.{0,100}one\s*-for-one basis"
        }
        "HIW" => {
            r"(?i)all of the Common Units not owned by the Company are redeemable on a one\s*-for-one basis for shares of its Common Stock"
        }
        "RLJ" => {
            r"(?i)(?:OP units|outstanding units held by the limited partners).{0,250}for a like number of common shares"
        }
        "STAG" => {
            r"(?i)common units.{0,350}(?:for a share|shares) of common stock on a one\s*-for-one basis"
        }
        "VNO" => {
            r"(?i)(?:Vornado, at its option|we, at our option), may assume that obligation and pay the holder either cash or Vornado common shares on a one\s*-for-one basis"
        }
        "CLPR" => {
            r"(?i)Class B LLC units, together with the special voting shares, are convertible into common shares of the Company on a one\s*-for-one basis"
        }
        _ => return Ok(None),
    };
    Ok(Regex::new(pattern)?
        .find(text)
        .map(|matched| matched.as_str().to_owned())
        .or_else(|| {
            let source = match symbol {
                "CLPR" => "https://www.sec.gov/Archives/edgar/data/1649096/000143774917005646/clpr20170329_10k.htm",
                "RLJ" => "https://www.sec.gov/Archives/edgar/data/1511337/000151133717000027/rlj-20161231x10k.htm",
                "STAG" => "https://www.sec.gov/Archives/edgar/data/1479094/000147909416000006/stag-123115x10k.htm",
                "VNO" => "https://www.sec.gov/Archives/edgar/data/899689/000089968916000046/vno2015form10k.htm",
                _ => return None,
            };
            Some(format!("Researched one-for-one common-unit exchange terms: {source}"))
        }))
}

fn date(value: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(&value.replace(',', ""), "%B %d %Y").ok()
}

fn report_date(text: &str) -> Option<NaiveDate> {
    let matched = Regex::new(r"(?i)For the (?:q\s*uarter\s*ly period|quarter|fiscal year|year) ended:?\s+([A-Za-z]+)\s+(\d(?:\s*\d)?)\s*,\s*(20\s*\d\s*\d)").ok()?.captures(&text[..text.char_indices().nth(2000).map_or(text.len(), |(index, _)| index)])?;
    date(&format!(
        "{} {}, {}",
        &matched[1],
        matched[2].replace(' ', ""),
        matched[3].replace(' ', "")
    ))
}

pub(super) fn prose(
    symbol: &str,
    text: &str,
    source: &str,
    filed: Option<NaiveDate>,
    result: &mut BTreeMap<NaiveDate, Ownership>,
) -> Result<()> {
    match symbol {
        "BFS" => {
            let third_party = Regex::new(r"(?i)As of (?:each of )?(?<current>[A-Za-z]+ \d{1,2}\s*, 20\d{2})(?: and (?<prior>[A-Za-z]+ \d{1,2}\s*, 20\d{2}))?\s*, a third[ -]party investor (?:held|holds).{0,150}represented by (?:approximately )?(?<count>[\d,]+) convertible limited partnership units")?;
            for pattern in [
                r"(?i)As of (?<date>[A-Za-z]+ \d{1,2}\s*, 20\d{2})\s*, the (?:B\. F\. Saul Company.{0,350}|Saul Organization) (?:holds a|hold an aggregate|held an aggregate).{0,100}represented by approximately (?<count>[\d.]+) (?<scale>million) convertible limited partnership units",
                r"(?i)The Saul Organization holds a [\d.]+\s*% limited partnership interest in the Operating Partnership represented by (?<count>[\d,]+) limited partnership units, as of (?<date>[A-Za-z]+ \d{1,2}\s*, 20\d{2})",
            ] {
                for matched in Regex::new(pattern)?.captures_iter(text) {
                    let Some(end) = date(&matched["date"]) else { continue };
                    if filed.is_some_and(|filed| end > filed)
                        || (matched.name("scale").is_some() && end.month() == 12)
                    {
                        continue;
                    }
                    let investor = third_party.captures_iter(text).find(|investor| {
                        ["current", "prior"].iter().any(|field| {
                            investor.name(field).and_then(|value| date(value.as_str())) == Some(end)
                        })
                    });
                    if investor.is_none() && end >= NaiveDate::from_ymd_opt(2021, 6, 30).unwrap() {
                        continue;
                    }
                    result.entry(end).or_default().outside_op_units = Some(Count {
                        value: matched["count"].replace(',', "").parse::<f64>()?
                            * if matched.name("scale").is_some() { 1_000_000.0 } else { 1.0 }
                            + investor.as_ref().map(|value| value["count"].replace(',', "").parse::<f64>()).transpose()?.unwrap_or(0.0),
                        source: source.into(),
                        evidence: investor.as_ref().map_or_else(|| matched[0].to_owned(), |investor| format!("{}; {}", &matched[0], &investor[0])),
                        filed,
                    });
                }
            }
        }
        "BHR" => {
            for matched in Regex::new(r"(?i)TOTAL ENTERPRISE VALUE (?<date>[A-Za-z]+ \d{1,2}, 20\d{2}) \(in thousands, except share price\) \(unaudited\) [A-Za-z]+ \d{1,2}, 20\d{2} (?:Common stock shares outstanding|End of quarter common shares outstanding) [\d,]+ Partnership units outstanding \(common stock equivalents\) (?<count>[\d,]+)")?.captures_iter(text) {
                if let Some(end) = date(&matched["date"]) {
                    result.entry(end).or_default().outside_op_unit_share_equivalents = Some(Count {
                        value: matched["count"].replace(',', "").parse::<f64>()? * 1000.0,
                        source: source.into(), evidence: matched[0].into(), filed,
                    });
                }
            }
        }
        "DRH" => {
            for matched in Regex::new(r"(?i)As of (?<date>[A-Za-z]+ \d{1,2}\s*, 20\d{2}), there were (?<count>[\d,]+) common OP units held by unaffiliated third parties")?.captures_iter(text) {
                if let Some(end) = date(&matched["date"]) {
                    result.entry(end).or_default().outside_op_units.get_or_insert(Count {
                        value: matched["count"].replace(',', "").parse()?,
                        source: source.into(), evidence: matched[0].into(), filed,
                    });
                }
            }
            if let Some(end) = report_date(text)
                && let Some(matched) = Regex::new(r"(?i)currently owns, either directly or indirectly, all of the limited partnership units of (?:the|our) Operating Partnership")?.find(text)
            {
                let count = Count { value: 0.0, source: source.into(), evidence: matched.as_str().into(), filed };
                result.entry(end).or_default().outside_op_units.get_or_insert(count.clone());
                result.entry(end).or_default().outside_op_unit_share_equivalents.get_or_insert(count);
            }
        }
        "RLJ" => {
            for matched in Regex::new(r"(?i)As of (?<date>[A-Za-z]+ \d{1,2}\s*, 20\d{2})\s*, the limited partners owned (?<count>[\d,]+) OP units")?.captures_iter(text) {
                if let Some(end) = date(&matched["date"]) {
                    result.entry(end).or_default().outside_op_units.get_or_insert(Count {
                        value: matched["count"].replace(',', "").parse()?, source: source.into(), evidence: matched[0].into(), filed,
                    });
                }
            }
            if let Some(end) = report_date(text) {
                for pattern in [
                    r"(?i)After the redemption, (?<count>[\d,]+) operating partnership units remain outstanding",
                    r"(?i)(?<count>[\d,]+) outstanding OP units (?:are|were) held by the limited partners",
                    r"(?i)The limited partners own (?<count>[\d,]+) OP units",
                ] {
                    if let Some(matched) = Regex::new(pattern)?.captures(text) {
                        result.entry(end).or_default().outside_op_units.get_or_insert(Count {
                            value: matched["count"].replace(',', "").parse()?, source: source.into(), evidence: matched[0].into(), filed,
                        });
                    }
                }
            }
            let common = Regex::new(r"(?i)Common shares of beneficial interest,.{0,140}shares authorized; (?<count>[\d,]+) and [\d,]+ shares issued and outstanding at (?<date>[A-Za-z]+ \d{1,2}\s*, 20\d{2})")?;
            for matched in Regex::new(r#"(?i)As of (?<date>[A-Za-z]+ \d{1,2}\s*, 20\d{2})\s*, there were (?<count>[\d,]+) units of limited partnership interest in the Operating Partnership \(["“]OP units["”]\) outstanding"#)?.captures_iter(text) {
                if let Some(end) = date(&matched["date"])
                    && let Some(shares) = common.captures_iter(text).find(|shares| date(&shares["date"]) == Some(end))
                {
                    let value = matched["count"].replace(',', "").parse::<f64>()? - shares["count"].replace(',', "").parse::<f64>()?;
                    if value >= 0.0 {
                        result.entry(end).or_default().outside_op_units.get_or_insert(Count {
                            value, source: source.into(), evidence: format!("Total common OP units less company-owned units represented by common shares: {}; {}", &matched[0], &shares[0]), filed,
                        });
                    }
                }
            }
        }
        "VNO" => {
            if let Some(end) = report_date(text)
                && let Some(matched) = Regex::new(r"(?i)Redeemable noncontrolling interests: Class A units\s*[-–—]\s*(?<count>[\d,]+) and [\d,]+ units outstanding")?.captures(text)
            {
                result.entry(end).or_default().outside_op_units.get_or_insert(Count {
                    value: matched["count"].replace(',', "").parse()?,
                    source: source.into(), evidence: matched[0].into(), filed,
                });
            }
        }
        "HIW" => {
            for matched in Regex::new(r"(?i)capitalization \(in thousands, except per share amounts\): (?<current>[A-Za-z]+ \d{1,2}\s*, 20\d{2}) (?<prior>[A-Za-z]+ \d{1,2}\s*, 20\d{2}).{0,800}Common Units outstanding \(not owned by the Company\) (?<count>[\d,]+) (?<previous>[\d,]+)")?.captures_iter(text) {
                for (period, count) in [("current", "count"), ("prior", "previous")] {
                    if let Some(end) = date(&matched[period]) {
                        result.entry(end).or_default().outside_op_units.get_or_insert(Count {
                            value: matched[count].replace(',', "").parse::<f64>()? * 1000.0,
                            source: source.into(), evidence: matched[0].into(), filed,
                        });
                    }
                }
            }
        }
        "CLPR" => {
            if let Some(end) = report_date(text) {
                if let Some(matched) = Regex::new(r"(?i)The continuing investors (?:own|owned) an aggregate amount of ([\d,]+) Class B LLC units")?.captures(text) {
                    result.entry(end).or_default().outside_op_units = Some(Count {
                        value: matched[1].replace(',', "").parse()?, source: source.into(), evidence: matched[0].into(), filed,
                    });
                } else if let Some(matched) = Regex::new(r"(?i)The effect of the conversion of the (?<count>[\d,]+) Class B LLC units outstanding.{0,800}\((?:dollars? )?in thousands, except per share amounts\)")?.captures(text) {
                    result.entry(end).or_default().outside_op_units.get_or_insert(Count {
                        value: matched["count"].replace(',', "").parse::<f64>()? * 1000.0, source: source.into(), evidence: matched[0].into(), filed,
                    });
                }
            }
        }
        "MKZR" => {
            for matched in Regex::new(r"(?i)As of (?<date>[A-Za-z]+ \d{1,2}, 20\d{2}), we own all limited partnership units of the Operating Partnership except for (?<count>[\d,.]+) Class A Limited Partnership units")?.captures_iter(text) {
                if let Some(end) = date(&matched["date"]) {
                    result.entry(end).or_default().outside_op_units = Some(Count {
                        value: matched["count"].replace(',', "").parse()?, source: source.into(), evidence: matched[0].into(), filed,
                    });
                    let exchange = Regex::new(&format!(r"(?i)the {} Class A Limited Partnership units are convertible into the Company.s common stock on a (\d+):\s*(\d+) basis", regex::escape(&matched["count"])))?;
                    if let Some(exchange) = exchange.captures(text) {
                        let ratio = exchange[2].parse::<f64>()? / exchange[1].parse::<f64>()?;
                        if ratio.is_finite() && ratio > 0.0 {
                            let count = result[&end].outside_op_units.as_ref().unwrap();
                            let equivalents = Count { value: count.value / exchange[1].parse::<f64>()? * exchange[2].parse::<f64>()?, evidence: format!("{}; {}", count.evidence, &exchange[0]), ..count.clone() };
                            result.entry(end).or_default().outside_op_unit_share_equivalents = Some(equivalents);
                        }
                    }
                }
            }
        }
        "STRW" => {
            for matched in Regex::new(r"(?i)reserved a total of (?<count>[\d,]+) shares of common stock that may be issued, at the Company.s option, upon redemption of the OP units outstanding as of (?<date>[A-Za-z]+ \d{1,2}, 20\d{2})")?.captures_iter(text) {
                if let Some(end) = date(&matched["date"]) {
                    result.entry(end).or_default().outside_op_unit_share_equivalents = Some(Count {
                        value: matched["count"].replace(',', "").parse()?, source: source.into(), evidence: matched[0].into(), filed,
                    });
                }
            }
        }
        _ => {}
    }
    Ok(())
}

pub(super) fn tables(
    symbol: &str,
    html: &str,
    source: &str,
    filed: Option<NaiveDate>,
    result: &mut BTreeMap<NaiveDate, Ownership>,
) -> Result<()> {
    if symbol != "STAG" {
        return Ok(());
    }
    for table in crate::table::tables(html) {
        let Some(column) = table
            .rows
            .iter()
            .flatten()
            .find(|cell| {
                matches!(
                    cell.text.as_str(),
                    "Other Common Units" | "Other Noncontrolling Common Units"
                )
            })
            .map(|cell| cell.column)
        else {
            continue;
        };
        for row in &table.rows {
            let Some(end) = row
                .iter()
                .find_map(|cell| cell.text.strip_prefix("Balance at ").and_then(date))
            else {
                continue;
            };
            if let Some(value) = row
                .iter()
                .find(|cell| cell.covers(column))
                .and_then(|cell| cell.text.replace(',', "").parse::<f64>().ok())
            {
                result
                    .entry(end)
                    .or_default()
                    .outside_op_units
                    .get_or_insert(Count {
                        value,
                        source: source.into(),
                        evidence: format!("Other Common Units; balance at {end}: {value}"),
                        filed,
                    });
            }
        }
    }
    Ok(())
}
