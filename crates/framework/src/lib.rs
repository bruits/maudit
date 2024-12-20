pub mod assets;
pub mod page;
pub mod routes;

use std::{
    fs::{self, File},
    io::Write,
    path::PathBuf,
    str::FromStr,
    time::SystemTime,
};

use colored::Colorize;
pub use dire_coronet_macros;
use env_logger::{Builder, Env};
pub use maud;

use log::{info, trace};

pub fn coronate(router: routes::Router) {
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
    trace!(target: "build", "Creating required directories...");
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

        // Write a file to this path
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
        let formatted_elasped_time = {
            let elapsed = route_start.elapsed().unwrap();
            match elapsed.as_secs() {
                secs if secs > 0 => format!("({}s)", secs).red(),
                _ => match elapsed.as_millis() {
                    millis if millis > 10 => format!("({}ms)", millis).yellow(),
                    millis if millis > 0 => format!("({}ms)", millis).normal(),
                    _ => format!("({}μs)", elapsed.as_micros()).dimmed(),
                },
            }
        };
        info!(target: "build", "{} -> {} {}", route.route(), file_path.to_string_lossy().dimmed(), formatted_elasped_time);
    }
    let formatted_elasped_time = {
        let elapsed = pages_start.elapsed().unwrap();
        match elapsed.as_secs() {
            secs if secs > 60 => format!("{}m", secs / 60).red(),
            secs if secs > 0 => format!("{}s", secs).yellow(),
            _ => match elapsed.as_millis() {
                millis if millis > 0 => format!("{}ms", millis).normal(),
                _ => format!("{}μs", elapsed.as_micros()).normal(),
            },
        }
    };
    info!(target: "build", "{}", format!("Pages generated in {}", formatted_elasped_time).bold());
}
