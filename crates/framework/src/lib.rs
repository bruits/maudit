pub mod assets;
pub mod page;
pub mod routes;

mod logging;

use std::{
    fs::{self, File},
    io::{self, Write},
    path::{Path, PathBuf},
    str::FromStr,
    time::SystemTime,
};

use colored::{ColoredString, Colorize};
pub use dire_coronet_macros;
use env_logger::{Builder, Env};
use logging::{format_elapsed_time, FormatElapsedTimeOptions};
pub use maud;

use log::{info, trace};

pub fn coronate(router: routes::Router) -> Result<(), Box<dyn std::error::Error>> {
    let logging_env = Env::default().filter_or("RUST_LOG", "info");
    Builder::from_env(logging_env)
        .format(|buf, record| {
            if record.target() == "SKIP_FORMAT" {
                return writeln!(buf, "{}", record.args());
            }

            writeln!(
                buf,
                "{} {} {}",
                chrono::Local::now().format("%H:%M:%S").to_string().dimmed(),
                format!("[{}]", record.target())
                    .to_string()
                    .to_ascii_lowercase()
                    .bright_yellow(),
                record.args()
            )
        })
        .init();

    println!("\n{}\n", "Let the coronation begin!".bold());

    // Create a directory for the output
    trace!(target: "build", "Setting up required directories...");
    fs::remove_dir_all("dist").unwrap_or_default();
    fs::create_dir_all("dist").unwrap();
    fs::create_dir_all("dist/_assets").unwrap();

    info!(target: "SKIP_FORMAT", "{}", " generating pages ".on_green().bold());
    let pages_start = SystemTime::now();

    for route in &router.routes {
        let route_start = SystemTime::now();
        let file_path = PathBuf::from_str("./dist/")
            .unwrap()
            .join(route.file_path());

        // Create the parent directories if it doesn't exist
        let parent_dir = Path::new(file_path.parent().unwrap());
        fs::create_dir_all(parent_dir)?;

        // Create file
        let mut file = File::create(file_path.clone()).unwrap();

        let rendered = route.render();
        match rendered {
            page::RenderResult::Html(html) => {
                file.write_all(html.into_string().as_bytes()).unwrap();
            }
            page::RenderResult::Text(text) => {
                file.write_all(text.as_bytes()).unwrap();
            }
        }

        let formatted_elasped_time = format_elapsed_time(
            route_start.elapsed(),
            FormatElapsedTimeOptions {
                additional_fn: Some(Box::new(|msg: ColoredString| {
                    let formatted_msg = format!("(+{})", msg);
                    if msg.fgcolor.is_none() {
                        formatted_msg.dimmed()
                    } else {
                        formatted_msg.into()
                    }
                })),
                ..Default::default()
            },
        )?;
        info!(target: "build", "{} -> {} {}", route.route(), file_path.to_string_lossy().dimmed(), formatted_elasped_time);
    }

    let formatted_elasped_time = format_elapsed_time(
        pages_start.elapsed(),
        FormatElapsedTimeOptions {
            sec_red_threshold: 5,
            sec_yellow_threshold: 1,
            ..Default::default()
        },
    )?;
    info!(target: "build", "{}", format!("Pages generated in {}", formatted_elasped_time).bold());

    // Check if static directory exists
    if PathBuf::from_str("./static").unwrap().exists() {
        let assets_start = SystemTime::now();
        info!(target: "SKIP_FORMAT", "{}", " copying assets ".on_green().bold());

        // Copy the static directory to the dist directory
        copy_recursively("./static", "./dist")?;

        let formatted_elasped_time =
            format_elapsed_time(assets_start.elapsed(), FormatElapsedTimeOptions::default())?;
        info!(target: "build", "{}", format!("Assets copied in {}", formatted_elasped_time).bold());
    }

    Ok(())
}

pub fn copy_recursively(source: impl AsRef<Path>, destination: impl AsRef<Path>) -> io::Result<()> {
    fs::create_dir_all(&destination)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let filetype = entry.file_type()?;
        if filetype.is_dir() {
            copy_recursively(entry.path(), destination.as_ref().join(entry.file_name()))?;
        } else {
            fs::copy(entry.path(), destination.as_ref().join(entry.file_name()))?;
        }
    }
    Ok(())
}
