use colored::{ColoredString, Colorize};
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

pub fn format_elapsed_time(
    elapsed: Result<Duration, SystemTimeError>,
    options: &FormatElapsedTimeOptions,
) -> Result<ColoredString, SystemTimeError> {
    let elapsed = match elapsed {
        Ok(elapsed) => elapsed,
        Err(err) => return Err(err),
    };

    let result = match elapsed.as_secs() {
        secs if secs > 60 => {
            let mins = secs / 60;
            let secs = secs % 60;
            format!("{}m{}s", mins, secs).red()
        }
        secs if secs > options.sec_red_threshold => format!("{}s", secs).red(),
        secs if secs > options.sec_yellow_threshold => format!("{}s", secs).yellow(),
        secs if secs > 0 => format!("{}s", secs).dimmed(),
        _ => match elapsed.as_millis() {
            millis
                if options
                    .millis_red_threshold
                    .map_or(false, |threshold| millis > threshold) =>
            {
                format!("{}ms", millis).red()
            }
            millis
                if options
                    .millis_yellow_threshold
                    .map_or(false, |threshold| millis > threshold) =>
            {
                format!("{}ms", millis).yellow()
            }
            millis if millis > 0 => format!("{}ms", millis).dimmed(),
            _ => format!("{}μs", elapsed.as_micros()).dimmed(),
        },
    };

    if let Some(additional_fn) = &options.additional_fn {
        Ok(additional_fn(result))
    } else {
        Ok(result)
    }
}
