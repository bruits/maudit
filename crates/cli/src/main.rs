mod build;
mod dev;
mod preview;

mod logging;

use clap::{Parser, Subcommand};
use colored::Colorize;
use dev::coordinate_dev_env;
use logging::init_logging;
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

#[tokio::main]
async fn main() {
    init_logging();
    let cli = Cli::parse();

    // You can check for the existence of subcommands, and if found use their
    // matches just as you would the top level cmd
    match &cli.command {
        Commands::Build {} => {
            build::start_build();
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
