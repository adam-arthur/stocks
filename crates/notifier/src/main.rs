mod delivery;
mod rules;
mod state;

use anyhow::{Context, Result, bail};
use chrono::Local;
use std::{env, path::Path};

const USAGE: &str = "Usage: notifier <evaluate|deliver>\nRun from the workspace root.";

fn main() -> Result<()> {
    let args: Vec<_> = env::args().skip(1).collect();
    let [command] = args.as_slice() else {
        bail!(USAGE)
    };
    match command.as_str() {
        "-h" | "--help" => {
            println!("{USAGE}");
            return Ok(());
        }
        "evaluate" | "deliver" => {}
        _ => bail!(USAGE),
    }
    let store = state::Store::open(Path::new("data/notifier")).context("opening data/notifier")?;
    let mut state = store.load()?;
    if command == "evaluate" {
        rules::evaluate(
            &mut state,
            Path::new("data/derived/stocks"),
            Local::now().date_naive(),
        )
        .context("evaluating notification rules")?;
        store.save(&state)?;
        eprintln!("{} pending notifications", state.pending.len());
        Ok(())
    } else {
        delivery::deliver(&store, &mut state, delivery::terminal)
    }
}
