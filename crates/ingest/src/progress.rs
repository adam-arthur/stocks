use std::{
    io::{self, IsTerminal},
    time::{Duration, Instant},
};

use indicatif::ProgressBar;

pub struct Progress {
    bar: ProgressBar,
    phase: &'static str,
    total: usize,
    done: usize,
    last_log: Instant,
    terminal: bool,
}

impl Progress {
    pub fn new(bar: &ProgressBar, phase: &'static str, total: usize, detail: &str) -> Self {
        let mut progress = Self {
            bar: bar.clone(),
            phase,
            total,
            done: 0,
            last_log: Instant::now(),
            terminal: io::stderr().is_terminal() && log::log_enabled!(log::Level::Info),
        };
        if progress.terminal {
            progress.bar.reset();
            progress.bar.enable_steady_tick(Duration::from_millis(100));
            progress.bar.reset_elapsed();
        }
        progress.show(detail, true);
        progress
    }

    pub fn advance(&mut self, count: usize, detail: &str) {
        self.done += count;
        self.show(detail, self.done == self.total);
    }

    fn show(&mut self, detail: &str, force: bool) {
        let message = format!(
            "{} {:>3}% · {}/{} symbols · {} remaining{}",
            self.phase,
            (self.done * 100).checked_div(self.total).unwrap_or(100),
            self.done,
            self.total,
            self.total.saturating_sub(self.done),
            if detail.is_empty() {
                String::new()
            } else {
                format!(" · {detail}")
            },
        );
        if self.terminal {
            self.bar.set_message(message);
        } else if force || self.last_log.elapsed() >= Duration::from_secs(10) {
            log::info!("{message}");
            self.last_log = Instant::now();
        }
    }

    pub fn finish(&self) {
        if self.terminal {
            log::info!("{}", self.bar.message());
        } else if self.done < self.total {
            log::info!(
                "{} stopped · {}/{} symbols saved · {} remaining",
                self.phase,
                self.done,
                self.total,
                self.total - self.done
            );
        }
        self.bar.disable_steady_tick();
        self.bar.finish_and_clear();
    }
}

impl Drop for Progress {
    fn drop(&mut self) {
        self.bar.disable_steady_tick();
        self.bar.finish_and_clear();
    }
}
