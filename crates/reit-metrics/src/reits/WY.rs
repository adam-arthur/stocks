use crate::companies::{Company, build};
use anyhow::Result;

pub fn company() -> Result<Company> {
    build((
        "WY",
        106535,
        &[
            "weyerhaeuser.com",
            "www.weyerhaeuser.com",
            "investor.weyerhaeuser.com",
        ],
        vec![],
        None,
    ))
}
