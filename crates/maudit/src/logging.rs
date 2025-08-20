use colored::{ColoredString, Colorize};
use env_logger::{Builder, Env};
use log::info;
use std::io::Write;
use std::time::{Duration, SystemTimeError};

pub struct FormatElapsedTimeOptions<'a> {
    pub(crate) sec_yellow_threshold: u64,
    pub(crate) sec_red_threshold: u64,
    pub(crate) millis_yellow_threshold: Option<u128>,
    pub(crate) millis_red_threshold: Option<u128>,
    pub(crate) additional_fn: Option<&'a (dyn Fn(ColoredString) -> ColoredString + Sync)>,
}

impl Default for FormatElapsedTimeOptions<'_> {
    fn default() -> Self {
        Self {
            sec_yellow_threshold: 1,
            sec_red_threshold: 2,
            millis_yellow_threshold: Some(100),
            millis_red_threshold: Some(500),
            additional_fn: None,
        }
    }
}

pub fn init_logging() {
    let logging_env = Env::default().filter_or("RUST_LOG", "info");
    Builder::from_env(logging_env)
        .format(|buf, record| {
            if std::env::args().any(|arg| arg == "--quiet") {
                return Ok(());
            }

            if record.target() == "SKIP_FORMAT" {
                return writeln!(buf, "{}", record.args());
            }

            // TODO: Add different formatting for warn, error, etc.

            writeln!(
                buf,
                "{} {} {}",
                chrono::Local::now().format("%H:%M:%S").to_string().dimmed(),
                record.target().to_ascii_lowercase().bold().bright_yellow(),
                record.args()
            )
        })
        .init();
}

pub fn format_elapsed_time(
    elapsed: Result<Duration, SystemTimeError>,
    options: &FormatElapsedTimeOptions,
) -> Result<ColoredString, SystemTimeError> {
    let elapsed = elapsed?;

    let result = match elapsed.as_secs() {
        secs if secs > options.sec_red_threshold => format!("{}m", secs / 60).red(),
        secs if secs > options.sec_yellow_threshold => format!("{}s", secs).yellow(),
        secs if secs > 0 => format!("{}s", secs).normal(),
        _ => match elapsed.as_millis() {
            millis
                if options
                    .millis_red_threshold
                    .is_some_and(|threshold| millis > threshold) =>
            {
                format!("{}ms", millis).red()
            }
            millis
                if options
                    .millis_yellow_threshold
                    .is_some_and(|threshold| millis > threshold) =>
            {
                format!("{}ms", millis).yellow()
            }
            millis if millis > 0 => format!("{}ms", millis).normal(),
            _ => format!("{}μs", elapsed.as_micros()).normal(),
        },
    };

    if let Some(additional_fn) = &options.additional_fn {
        Ok(additional_fn(result))
    } else {
        Ok(result)
    }
}

pub fn print_title(title: &str) {
    info!(target: "SKIP_FORMAT", "{}", "");
    info!(target: "SKIP_FORMAT", "{}", format!(" {} ", title).on_green().bold());
}
