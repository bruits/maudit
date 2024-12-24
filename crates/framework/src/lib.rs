pub mod assets;
pub mod page;
pub mod params;
pub mod routes;

mod logging;

use std::{
    collections::{HashMap, HashSet},
    fs::{self, File},
    io::{self, Write},
    path::{Path, PathBuf},
    str::FromStr,
    time::SystemTime,
};

use assets::Asset;
use colored::{ColoredString, Colorize};
use env_logger::{Builder, Env};

use logging::{format_elapsed_time, FormatElapsedTimeOptions};

pub use maud;
pub use maudit_macros;

use log::{info, trace};
use page::{RouteContext, RouteParams};

pub fn coronate(router: routes::Router) -> Result<(), Box<dyn std::error::Error>> {
    let build_start = SystemTime::now();
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

    // Create a directory for the output
    trace!(target: "build", "Setting up required directories...");
    fs::remove_dir_all("dist").unwrap_or_default();
    fs::create_dir_all("dist").unwrap();
    fs::create_dir_all("dist/_assets").unwrap();

    info!(target: "SKIP_FORMAT", "{}", " generating pages ".on_green().bold());
    let pages_start = SystemTime::now();

    let route_format_options = FormatElapsedTimeOptions {
        additional_fn: Some(&|msg: ColoredString| {
            let formatted_msg = format!("(+{})", msg);
            if msg.fgcolor.is_none() {
                formatted_msg.dimmed()
            } else {
                formatted_msg.into()
            }
        }),
        ..Default::default()
    };

    let section_format_options = FormatElapsedTimeOptions {
        sec_red_threshold: 5,
        sec_yellow_threshold: 1,
        millis_red_threshold: None,
        millis_yellow_threshold: None,
        ..Default::default()
    };

    let mut build_pages_assets: HashSet<Box<dyn Asset>> = HashSet::new();

    for route in &router.routes {
        let routes = route.routes();
        match routes.is_empty() {
            true => {
                let route_start = SystemTime::now();
                let mut page_assets = assets::PageAssets::default();
                let mut ctx = RouteContext {
                    params: page::RouteParams(HashMap::new()),
                    assets: &mut page_assets,
                };

                let (file_path, file) = create_route_file(&**route, &ctx.params)?;
                render_route(file, &**route, &mut ctx)?;

                let formatted_elasped_time =
                    format_elapsed_time(route_start.elapsed(), &route_format_options)?;
                info!(target: "build", "{} -> {} {}", route.route(&ctx.params), file_path.to_string_lossy().dimmed(), formatted_elasped_time);

                build_pages_assets.extend(page_assets.0);
            }
            false => {
                info!(target: "build", "{}", route.route_raw().to_string().bold());

                // Reuse the same PageAssets HashSet for all the routes of a dynamic page
                let mut pages_assets = assets::PageAssets::default();

                routes.into_iter().for_each(|params| {
                    let route_start = SystemTime::now();
                    let mut ctx = RouteContext { params, assets: &mut pages_assets };

                    let (file_path, file) = create_route_file(&**route, &ctx.params).unwrap();
                    render_route(file, &**route, &mut ctx).unwrap();

                    let formatted_elasped_time = format_elapsed_time(route_start.elapsed(), &route_format_options).unwrap();
                    info!(target: "build", "├─ {} {}", file_path.to_string_lossy().dimmed(), formatted_elasped_time);
                });

                build_pages_assets.extend(pages_assets.0);
            }
        }
    }

    let formatted_elasped_time =
        format_elapsed_time(pages_start.elapsed(), &section_format_options)?;
    info!(target: "build", "{}", format!("Pages generated in {}", formatted_elasped_time).bold());

    let assets_start = SystemTime::now();
    info!(target: "SKIP_FORMAT", "{}", format!(" generating {} assets ", build_pages_assets.len()).on_green().bold());

    build_pages_assets.iter().for_each(|asset| {
        asset.process();
    });

    let formatted_elasped_time =
        format_elapsed_time(assets_start.elapsed(), &section_format_options)?;
    info!(target: "build", "{}", format!("Assets generated in {}", formatted_elasped_time).bold());

    // Check if static directory exists
    if PathBuf::from_str("./static").unwrap().exists() {
        let assets_start = SystemTime::now();
        info!(target: "SKIP_FORMAT", "{}", " copying assets ".on_green().bold());

        // Copy the static directory to the dist directory
        copy_recursively("./static", "./dist")?;

        let formatted_elasped_time =
            format_elapsed_time(assets_start.elapsed(), &FormatElapsedTimeOptions::default())?;
        info!(target: "build", "{}", format!("Assets copied in {}", formatted_elasped_time).bold());
    }

    let formatted_elasped_time =
        format_elapsed_time(build_start.elapsed(), &section_format_options)?;
    info!(target: "build", "{}", format!("Build completed in {}", formatted_elasped_time).bold());

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

fn create_route_file(
    route: &dyn page::FullPage,
    params: &RouteParams,
) -> Result<(PathBuf, File), Box<dyn std::error::Error>> {
    let file_path = PathBuf::from_str("./dist/")
        .unwrap()
        .join(route.file_path(params));

    // Create the parent directories if it doesn't exist
    let parent_dir = Path::new(file_path.parent().unwrap());
    fs::create_dir_all(parent_dir)?;

    // Create file
    let file = File::create(file_path.clone())?;

    Ok((file_path, file))
}

fn render_route(
    mut file: File,
    route: &dyn page::FullPage,
    ctx: &mut RouteContext,
) -> Result<(), Box<dyn std::error::Error>> {
    let rendered = route.render(ctx);
    match rendered {
        page::RenderResult::Html(html) => {
            file.write_all(html.into_string().as_bytes())?;
        }
        page::RenderResult::Text(text) => {
            file.write_all(text.as_bytes())?;
        }
    }
    Ok(())
}
