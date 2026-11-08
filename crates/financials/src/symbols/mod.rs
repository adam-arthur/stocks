use crate::Rule;

mod aat;
mod adc;
mod ahr;
mod alx;
mod brt;
mod bxp;
mod cci;
mod clpr;
mod cold;
mod cube;
mod dlr;
mod elme;
mod els;
mod esrt;
mod exr;
mod fr;
mod frt;
mod fsp;
mod gipr;
mod gnl;
mod hiw;
mod hst;
mod iht;
mod iipr;
mod ivt;
mod jan;
mod jbgs;
mod kim;
mod krc;
mod maa;
mod nhp;
mod nlcp;
mod nnn;
mod o;
mod ohi;
mod olp;
mod self_storage;
mod sui;
mod svc;
mod ue;
mod uht;
mod umh;
mod whlr;
mod wy;

pub(crate) fn rules(symbol: &str) -> Vec<Rule> {
    match symbol {
        "AAT" => aat::rules(),
        "ADC" => adc::rules(),
        "AHR" => ahr::rules(),
        "ALX" => alx::rules(),
        "BRT" => brt::rules(),
        "BXP" => bxp::rules(),
        "CCI" => cci::rules(),
        "CLPR" => clpr::rules(),
        "COLD" => cold::rules(),
        "CUBE" => cube::rules(),
        "DLR" => dlr::rules(),
        "ELME" => elme::rules(),
        "ELS" => els::rules(),
        "ESRT" => esrt::rules(),
        "EXR" => exr::rules(),
        "FR" => fr::rules(),
        "FRT" => frt::rules(),
        "FSP" => fsp::rules(),
        "GIPR" => gipr::rules(),
        "GNL" => gnl::rules(),
        "HIW" => hiw::rules(),
        "HST" => hst::rules(),
        "IHT" => iht::rules(),
        "IIPR" => iipr::rules(),
        "IVT" => ivt::rules(),
        "JAN" => jan::rules(),
        "JBGS" => jbgs::rules(),
        "KIM" => kim::rules(),
        "KRC" => krc::rules(),
        "MAA" => maa::rules(),
        "NHP" => nhp::rules(),
        "NLCP" => nlcp::rules(),
        "NNN" => nnn::rules(),
        "O" => o::rules(),
        "OHI" => ohi::rules(),
        "OLP" => olp::rules(),
        "SELF" => self_storage::rules(),
        "SUI" => sui::rules(),
        "SVC" => svc::rules(),
        "UE" => ue::rules(),
        "UHT" => uht::rules(),
        "UMH" => umh::rules(),
        "WHLR" => whlr::rules(),
        "WY" => wy::rules(),
        _ => Vec::new(),
    }
}
