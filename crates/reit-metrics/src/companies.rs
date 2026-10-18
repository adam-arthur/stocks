use anyhow::{Result, bail};
use regex::Regex;

use crate::{Extraction, Metric, Observation, Unit};

pub(crate) type Supplements = for<'a> fn(
    &'a crate::source::Sources,
    &'a Company,
    chrono::NaiveDate,
    chrono::NaiveDate,
) -> std::pin::Pin<
    Box<dyn std::future::Future<Output = Result<Vec<(chrono::NaiveDate, String)>>> + Send + 'a>,
>;

pub struct Rule {
    pub metric: Metric,
    pub unit: Unit,
    pub label: Regex,
    pub scale: Option<u64>,
    pub section: Option<Regex>,
}

pub struct Company {
    pub symbol: &'static str,
    pub cik: u64,
    pub hosts: &'static [&'static str],
    pub filings: &'static [&'static str],
    pub rules: Vec<Rule>,
    pub(crate) tables: fn(&str) -> Vec<crate::table::Table>,
    pub(crate) extra_observations: Option<fn(&str, &str) -> Result<Vec<Observation>>>,
    pub(crate) preserve_units: bool,
    pub(crate) derive: Option<fn(&[Extraction]) -> Vec<Extraction>>,
    pub(crate) official_url: fn(&reqwest::Url) -> bool,
    pub(crate) additional_items: Option<fn(&str) -> bool>,
    pub(crate) supplements: Option<Supplements>,
    pub(crate) supplement_period_required: bool,
    pub(crate) pdf: Option<crate::pdf::Rules>,
    pub continuation: Option<&'static str>,
}

pub const SYMBOLS: &[&str] = &[
    "FSP", "FVR", "GIPR", "LAMR", "LAND",
    "REG", "REXR", "RHP", "RLJ", "RYN",
    "CCI", "CDP", "CHCT", "CLDT", "CLPR",
    "GLPI", "GNL", "GOOD", "GTY", "HIW",
    "SAFE", "SBAC", "SBRA", "SELF", "SHO",
    "BSRT",
    "BRT", "BRX", "BXP", "CBL",
    "SITC", "SKT", "SLG", "SMA", "SPG",
    "HPP", "HR", "HST", "IHT", "IIPR",
    "SQFT", "STRW", "SUI", "SVC", "TRNO", "ILPT", "INN", "IRM", "IRT", "IVT", "UDR", "UE", "UHT",
    "UMH", "VNO", "JAN", "JBGS", "KIM", "KRC", "KRG", "BDN", "BFS", "BHM", "BHR", "BNL", "AAT",
    "ADC", "AHR", "AHT", "AIV", "O", "PINE", "PSA", "PLD", "AMT", "EQIX", "DLR", "VICI", "STAG",
    "INVH", "WELL", "AKR", "ALX", "AMH", "APLE", "ARE", "VTR", "WHLR", "WPC", "WY", "XHR",
];

pub fn company(symbol: &str) -> Result<Company> {
    match symbol {
        "FSP" => crate::reits::fsp::company(),
        "FVR" => crate::reits::fvr::company(),
        "GIPR" => crate::reits::gipr::company(),
        "LAMR" => crate::reits::lamr::company(),
        "LAND" => crate::reits::land::company(),

        "REG" => crate::reits::reg::company(),
        "REXR" => crate::reits::rexr::company(),
        "RHP" => crate::reits::rhp::company(),
        "RLJ" => crate::reits::rlj::company(),
        "RYN" => crate::reits::ryn::company(),

        "CDP" => crate::reits::cdp::company(),
        "CHCT" => crate::reits::chct::company(),
        "CLDT" => crate::reits::cldt::company(),
        "CLPR" => crate::reits::clpr::company(),
        "CCI" => crate::reits::cci::company(),

        "GLPI" => crate::reits::glpi::company(),
        "GNL" => crate::reits::gnl::company(),
        "GOOD" => crate::reits::good::company(),
        "GTY" => crate::reits::gty::company(),
        "HIW" => crate::reits::hiw::company(),

        "SAFE" => crate::reits::safe::company(),
        "SBAC" => crate::reits::sbac::company(),
        "SBRA" => crate::reits::sbra::company(),
        "SELF" => crate::reits::self_storage::company(),
        "SHO" => crate::reits::sho::company(),

        "BSRT" => crate::reits::bsrt::company(),
        "BRT" => crate::reits::brt::company(),
        "BRX" => crate::reits::brx::company(),
        "BXP" => crate::reits::bxp::company(),
        "CBL" => crate::reits::cbl::company(),

        "SITC" => crate::reits::sitc::company(),
        "SKT" => crate::reits::skt::company(),
        "SLG" => crate::reits::slg::company(),
        "SMA" => crate::reits::sma::company(),
        "SPG" => crate::reits::spg::company(),

        "HPP" => crate::reits::hpp::company(),
        "HR" => crate::reits::hr::company(),
        "HST" => crate::reits::hst::company(),
        "IIPR" => crate::reits::iipr::company(),
        "IHT" => crate::reits::iht::company(),

        "SQFT" => crate::reits::sqft::company(),
        "STRW" => crate::reits::strw::company(),
        "SUI" => crate::reits::sui::company(),
        "SVC" => crate::reits::svc::company(),
        "TRNO" => crate::reits::trno::company(),

        "ILPT" => crate::reits::ilpt::company(),
        "INN" => crate::reits::inn::company(),
        "IRM" => crate::reits::irm::company(),
        "IRT" => crate::reits::irt::company(),
        "IVT" => crate::reits::ivt::company(),

        "UDR" => crate::reits::udr::company(),
        "UE" => crate::reits::ue::company(),
        "UHT" => crate::reits::uht::company(),
        "UMH" => crate::reits::umh::company(),
        "VNO" => crate::reits::vno::company(),

        "JAN" => crate::reits::jan::company(),
        "JBGS" => crate::reits::jbgs::company(),
        "KIM" => crate::reits::kim::company(),
        "KRC" => crate::reits::krc::company(),
        "KRG" => crate::reits::krg::company(),

        "BDN" => crate::reits::bdn::company(),
        "BFS" => crate::reits::bfs::company(),
        "BHM" => crate::reits::bhm::company(),
        "BHR" => crate::reits::bhr::company(),
        "BNL" => crate::reits::bnl::company(),
        "VTR" => crate::reits::vtr::company(),
        "WHLR" => crate::reits::whlr::company(),
        "WPC" => crate::reits::wpc::company(),
        "WY" => crate::reits::wy::company(),
        "XHR" => crate::reits::xhr::company(),
        "ARE" => crate::reits::are::company(),
        "APLE" => crate::reits::aple::company(),
        "AMH" => crate::reits::amh::company(),
        "ALX" => crate::reits::alx::company(),
        "AKR" => crate::reits::akr::company(),
        "AIV" => crate::reits::aiv::company(),
        "AHT" => crate::reits::aht::company(),
        "AHR" => crate::reits::ahr::company(),
        "ADC" => crate::reits::adc::company(),
        "AAT" => crate::reits::aat::company(),
        "DLR" => crate::reits::dlr::company(),
        "VICI" => crate::reits::vici::company(),
        "INVH" => crate::reits::invh::company(),
        "STAG" => crate::reits::stag::company(),
        "WELL" => crate::reits::well::company(),
        "O" => crate::reits::o::company(),
        "PINE" => crate::reits::pine::company(),
        "PSA" => crate::reits::psa::company(),
        "PLD" => crate::reits::pld::company(),
        "AMT" => crate::reits::amt::company(),
        "EQIX" => crate::reits::eqix::company(),
        _ => bail!(
            "{symbol}: no researched extraction rules; supported: {}",
            SYMBOLS.join(", ")
        ),
    }
}

type Definition = (
    &'static str,
    u64,
    &'static [&'static str],
    Vec<(Metric, Unit, &'static str, u64)>,
    Option<&'static str>,
);

pub(crate) fn build((symbol, cik, hosts, patterns, continuation): Definition) -> Result<Company> {
    Ok(Company {
        symbol,
        cik,
        hosts,
        filings: &["8-K", "8-K/A"],
        continuation,
        tables: crate::table::tables,
        extra_observations: None,
        preserve_units: false,
        derive: None,
        official_url: |_| false,
        additional_items: None,
        supplements: None,
        supplement_period_required: false,
        pdf: None,
        rules: patterns
            .into_iter()
            .map(|(metric, unit, label, scale)| {
                Ok(Rule {
                    metric,
                    unit,
                    label: Regex::new(label)?,
                    scale: (scale > 0).then_some(scale),
                    section: None,
                })
            })
            .collect::<Result<_>>()?,
    })
}
