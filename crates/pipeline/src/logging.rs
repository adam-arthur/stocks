use std::io::{self, Write};

use indicatif::{ProgressBar, ProgressStyle};

pub fn init() -> ProgressBar {
    let bar = ProgressBar::new_spinner();
    bar.set_style(ProgressStyle::with_template("{spinner} {msg} [{elapsed_precise}]").unwrap());
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp(None)
        .format_target(false)
        .target(env_logger::Target::Pipe(Box::new(LogWriter(bar.clone()))))
        .init();
    bar
}

struct LogWriter(ProgressBar);

impl Write for LogWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.suspend(|| io::stderr().write(bytes))
    }

    fn flush(&mut self) -> io::Result<()> {
        io::stderr().flush()
    }
}
