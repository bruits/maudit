use colored::{ColoredString, Colorize};
use std::{
    fmt,
    io::Write,
    time::{Duration, SystemTimeError},
};
use tracing::{Event, Subscriber};
use tracing_subscriber::{
    fmt::{format, FmtContext, FormatEvent, FormatFields},
    layer::SubscriberExt,
    registry::LookupSpan,
    util::SubscriberInitExt,
};

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
    let elapsed = elapsed?;

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

pub struct EventLoggerFormatter;

impl<S, N> FormatEvent<S, N> for EventLoggerFormatter
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
{
    fn format_event(
        &self,
        ctx: &FmtContext<'_, S, N>,
        mut writer: format::Writer<'_>,
        event: &Event<'_>,
    ) -> fmt::Result {
        if std::env::args().any(|arg| arg == "--quiet") {
            return Ok(());
        }

        if event.metadata().name() == "SKIP_FORMAT" {
            ctx.field_format().format_fields(writer.by_ref(), event)?;
            return writeln!(writer);
        }

        // TODO: Add different formatting for warn, error, etc.

        let timestamp = chrono::Local::now().format("%H:%M:%S").to_string().dimmed();
        let event_name = event.metadata().name();

        write!(
            writer,
            "{}{} ",
            timestamp,
            if event_name.is_empty() {
                String::new()
            } else {
                format!(
                    " {}",
                    event_name.to_ascii_lowercase().bold().bright_yellow()
                )
            }
        )?;

        // Write fields on the event
        ctx.field_format().format_fields(writer.by_ref(), event)?;

        if *event.metadata().level() == tracing::Level::ERROR {
            // Write the writer to a string so we can colorize it
        }

        writeln!(writer)
    }
}

pub fn init_logging() {
    let tracing_formatter = tracing_subscriber::fmt::layer().event_format(EventLoggerFormatter);

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                format!("{}=info,tower_http=info", env!("CARGO_CRATE_NAME")).into()
            }),
        )
        .with(tracing_formatter)
        .init();
}
