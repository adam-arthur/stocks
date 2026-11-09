use std::{collections::BTreeMap, path::Path};

use anyhow::{Context, Result, ensure};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Debt {
    as_of: NaiveDate,
    total_debt_principal: u64,
    maturities: BTreeMap<i32, u64>,
    thereafter: u64,
}

pub fn load(path: &Path) -> Result<Option<Value>> {
    super::read::<Debt>(path)?
        .map(|debt| {
            ensure!(
                !debt.maturities.is_empty(),
                "debt snapshot has no maturity buckets: {}",
                path.display()
            );
            ensure!(
                debt.maturities
                    .values()
                    .try_fold(debt.thereafter, |sum, amount| sum.checked_add(*amount))
                    .context("debt maturity total overflow")?
                    == debt.total_debt_principal,
                "debt maturity buckets do not reconcile to total principal: {}",
                path.display()
            );
            Ok(serde_json::to_value(debt)?)
        })
        .transpose()
}
