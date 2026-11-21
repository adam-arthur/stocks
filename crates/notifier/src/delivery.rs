use crate::state::{Alert, Notification, State, Store};
use anyhow::Result;
use std::io::{self, Write};

pub fn terminal(notification: &Notification) -> Result<()> {
    let mut output = io::stdout().lock();
    let alerts = match notification {
        Notification::InitialSummary(alerts) => {
            writeln!(output, "Initial matches ({})", alerts.len())?;
            alerts.as_slice()
        }
        Notification::Entered(alert) => std::slice::from_ref(alert),
    };
    for Alert {
        symbol,
        observed_on,
        value,
        percentile,
        std_devs_from_median,
        ..
    } in alerts
    {
        writeln!(
            output,
            "{observed_on}  {symbol}  TTM yield {:.3}%  |  5-year percentile {:.2}  |  SD from median {}",
            value * 100.0,
            percentile,
            std_devs_from_median.map_or_else(|| "N/A".into(), |value| format!("{value:+.2}"))
        )?;
    }
    Ok(output.flush()?)
}

pub fn deliver(
    store: &Store,
    state: &mut State,
    mut send: impl FnMut(&Notification) -> Result<()>,
) -> Result<()> {
    while let Some(notification) = state.pending.front() {
        send(notification)?;
        state.pending.pop_front();
        store.save(state)?;
    }
    Ok(())
}
