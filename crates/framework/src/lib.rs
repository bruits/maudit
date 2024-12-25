// Modules the end-user will interact directly or indirectly with
mod assets;
pub mod page;
pub mod params;

// Re-exported dependencies for user convenience
pub use rustc_hash::FxHashMap;

pub use maudit_macros::generate_pages_mod;

// Internal modules
mod logging;

use std::{
    fs::{self, File},
    io::{self, Write},
    path::{Path, PathBuf},
    process::Termination,
    str::FromStr,
    time::SystemTime,
};

use colored::{ColoredString, Colorize};
use env_logger::{Builder, Env};
use log::{info, trace};
use page::{FullPage, RouteContext, RouteParams};
use rolldown::{Bundler, BundlerOptions, InputItem};
use rustc_hash::FxHashSet;

use assets::Asset;
use logging::{format_elapsed_time, FormatElapsedTimeOptions};

#[macro_export]
macro_rules! routes {
    [$($route:ident),*] => {
        vec![$(&$route),*]
    };
}

#[derive(Debug)]
pub struct PageOutput {
    pub route: String,
    pub file_path: String,
    pub params: Option<FxHashMap<String, String>>,
}

#[derive(Debug)]
pub struct StaticAssetOutput {
    pub file_path: String,
    pub original_path: String,
}

#[derive(Debug)]
pub struct BuildOutput {
    pub start_time: SystemTime,
    pub pages: Vec<PageOutput>,
    pub assets: Vec<String>,
    pub static_files: Vec<StaticAssetOutput>,
}

impl Termination for BuildOutput {
    fn report(self) -> std::process::ExitCode {
        0.into()
    }
}

pub fn coronate(routes: Vec<&dyn FullPage>) -> Result<BuildOutput, Box<dyn std::error::Error>> {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async { build(routes).await })
}

pub async fn build(routes: Vec<&dyn FullPage>) -> Result<BuildOutput, Box<dyn std::error::Error>> {
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

    let mut build_metadata = BuildOutput {
        start_time: build_start,
        pages: Vec::new(),
        assets: Vec::new(),
        static_files: Vec::new(),
    };

    let mut build_pages_assets: FxHashSet<Box<dyn Asset>> = FxHashSet::default();
    let mut build_pages_scripts: FxHashSet<assets::Script> = FxHashSet::default();

    for route in routes {
        let routes = route.routes();
        match routes.is_empty() {
            true => {
                let route_start = SystemTime::now();
                let mut page_assets = assets::PageAssets::default();
                let mut ctx = RouteContext {
                    params: page::RouteParams(FxHashMap::default()),
                    assets: &mut page_assets,
                };

                let (file_path, file) = create_route_file(route, &ctx.params)?;
                render_route(file, route, &mut ctx)?;

                let formatted_elasped_time =
                    format_elapsed_time(route_start.elapsed(), &route_format_options)?;
                info!(target: "build", "{} -> {} {}", route.route(&ctx.params), file_path.to_string_lossy().dimmed(), formatted_elasped_time);

                build_pages_assets.extend(page_assets.assets);
                build_pages_scripts.extend(page_assets.scripts);

                build_metadata.pages.push(PageOutput {
                    route: route.route_raw().to_string(),
                    file_path: file_path.to_string_lossy().to_string(),
                    params: None,
                });
            }
            false => {
                info!(target: "build", "{}", route.route_raw().to_string().bold());

                // Reuse the same PageAssets HashSet for all the routes of a dynamic page
                let mut pages_assets = assets::PageAssets::default();

                routes.into_iter().for_each(|params| {
                    let route_start = SystemTime::now();
                    let mut ctx = RouteContext { params, assets: &mut pages_assets };

                    let (file_path, file) = create_route_file(route, &ctx.params).unwrap();
                    render_route(file, route, &mut ctx).unwrap();

                    let formatted_elasped_time = format_elapsed_time(route_start.elapsed(), &route_format_options).unwrap();
                    info!(target: "build", "├─ {} {}", file_path.to_string_lossy().dimmed(), formatted_elasped_time);

                    build_metadata.pages.push(PageOutput {
                        route: route.route_raw(),
                        file_path: file_path.to_string_lossy().to_string(),
                        params: Some(ctx.params.0.clone()),
                    });
                });

                build_pages_assets.extend(pages_assets.assets);
                build_pages_scripts.extend(pages_assets.scripts);
            }
        }
    }

    let formatted_elasped_time =
        format_elapsed_time(pages_start.elapsed(), &section_format_options)?;
    info!(target: "build", "{}", format!("Pages generated in {}", formatted_elasped_time).bold());

    let assets_start = SystemTime::now();
    info!(target: "SKIP_FORMAT", "{}", " generating assets ".on_green().bold());

    build_pages_assets.iter().for_each(|asset| {
        asset.process();

        // TODO: Add outputted assets to build_metadata, might need dedicated fs methods for this
    });

    let bundler_inputs = build_pages_scripts
        .iter()
        .map(|script| InputItem {
            import: script.path().to_string_lossy().to_string(),
            ..Default::default()
        })
        .collect::<Vec<InputItem>>();

    if !bundler_inputs.is_empty() {
        let mut bundler = Bundler::new(BundlerOptions {
            input: Some(bundler_inputs),
            minify: Some(true),
            dir: Some("dist/_assets".to_string()),

            ..Default::default()
        });

        let _result = bundler.write().await.unwrap();

        // TODO: Add outputted chunks to build_metadata
    }

    let formatted_elasped_time =
        format_elapsed_time(assets_start.elapsed(), &section_format_options)?;
    info!(target: "build", "{}", format!("Assets generated in {}", formatted_elasped_time).bold());

    // Check if static directory exists
    if PathBuf::from_str("./static").unwrap().exists() {
        let assets_start = SystemTime::now();
        info!(target: "SKIP_FORMAT", "{}", " copying assets ".on_green().bold());

        // Copy the static directory to the dist directory
        copy_recursively("./static", "./dist", &mut build_metadata)?;

        let formatted_elasped_time =
            format_elapsed_time(assets_start.elapsed(), &FormatElapsedTimeOptions::default())?;
        info!(target: "build", "{}", format!("Assets copied in {}", formatted_elasped_time).bold());
    }

    let formatted_elasped_time =
        format_elapsed_time(build_start.elapsed(), &section_format_options)?;
    info!(target: "build", "{}", format!("Build completed in {}", formatted_elasped_time).bold());

    Ok(build_metadata)
}

fn copy_recursively(
    source: impl AsRef<Path>,
    destination: impl AsRef<Path>,
    build_metadata: &mut BuildOutput,
) -> io::Result<()> {
    fs::create_dir_all(&destination)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let filetype = entry.file_type()?;
        if filetype.is_dir() {
            copy_recursively(
                entry.path(),
                destination.as_ref().join(entry.file_name()),
                build_metadata,
            )?;
        } else {
            fs::copy(entry.path(), destination.as_ref().join(entry.file_name()))?;
            build_metadata.static_files.push(StaticAssetOutput {
                original_path: entry.path().to_string_lossy().to_string(),
                file_path: destination
                    .as_ref()
                    .join(entry.file_name())
                    .to_string_lossy()
                    .to_string(),
            });
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
