mod dev;

use clap::{Parser, Subcommand};
use dev::coordinate_dev_env;

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
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    // You can check for the existence of subcommands, and if found use their
    // matches just as you would the top level cmd
    match &cli.command {
        Commands::Build {} => {
            println!("Building...");
        }
        Commands::Dev {} => {
            let _ = coordinate_dev_env(".".to_string()).await;
        }
    }
}
