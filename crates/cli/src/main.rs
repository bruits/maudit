mod dev;
mod logging;
mod preview;

use clap::{Parser, Subcommand};
use colored::Colorize;
use dev::coordinate_dev_env;
use preview::start_preview_web_server;
use std::fmt::{self};
use std::path::{Path, PathBuf};
use tracing::{Event, Subscriber};
use tracing_subscriber::fmt::format;
use tracing_subscriber::fmt::{format::FormatFields, FmtContext};
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{fmt::FormatEvent, layer::SubscriberExt};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Build the project
    Build,
    /// Run the project in development mode
    Dev,
    /// Preview the project
    Preview,
}

struct MyFormatter;

impl<S, N> FormatEvent<S, N> for MyFormatter
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

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let tracing_formatter = tracing_subscriber::fmt::layer().event_format(MyFormatter);

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                format!("{}=info,tower_http=info", env!("CARGO_CRATE_NAME")).into()
            }),
        )
        .with(tracing_formatter)
        .init();

    // You can check for the existence of subcommands, and if found use their
    // matches just as you would the top level cmd
    match &cli.command {
        Commands::Build {} => {
            todo!();
        }
        Commands::Preview {} => {
            // TODO: Dist path is hardcoded for now. Ideally, Maudit should output some kind of metadata file that can be read by the CLI.
            // (and then we could error on that instead of the dist path, ha)
            let dist_path = Path::new("dist");
            if !dist_path.exists() {
                println!(
                    "The dist directory does not exist. Please run `maudit build` or `cargo build` first."
                );
                return;
            }

            let _ = start_preview_web_server(PathBuf::from("dist")).await;
        }
        Commands::Dev {} => {
            let _ = coordinate_dev_env(".").await;
        }
    }
}
