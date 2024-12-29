// Modules the end-user will interact directly or indirectly with
mod assets;
pub mod content;
pub mod errors;
pub mod page;
pub mod params;

use content::ContentSources;
use errors::BuildError;
use maud::{html, Markup};
// Re-exported dependencies for user convenience
pub use rustc_hash::FxHashMap;

// Internal modules
mod logging;

use std::{
    fs::{self, remove_dir_all, File},
    io::{self, Write},
    path::{Path, PathBuf},
    process::Termination,
    str::FromStr,
    time::SystemTime,
};

use colored::{ColoredString, Colorize};
use env_logger::{Builder, Env};
use log::{info, trace};
use page::{DynamicRouteContext, FullPage, RenderResult, RouteContext, RouteParams};
use rolldown::{Bundler, BundlerOptions, InputItem};
use rustc_hash::FxHashSet;

use assets::Asset;
use logging::{format_elapsed_time, FormatElapsedTimeOptions};

use lol_html::{element, rewrite_str, RewriteStrSettings};

#[macro_export]
macro_rules! routes {
    [$($route:path),*] => {
        vec![$(&$route),*]
    };
}

#[macro_export]
macro_rules! content_sources {
    ($($source:expr),*) => {
        maudit::content::ContentSources(vec![$(Box::new($source)),*])
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

pub const GENERATOR: &str = concat!("Maudit v", env!("CARGO_PKG_VERSION"));
pub fn generator() -> Markup {
    html! {
        meta name="generator" content=(GENERATOR);
    }
}

pub struct BuildOptions {
    pub output_dir: String,
    pub assets_dir: String,
    pub static_dir: String,
}

impl Default for BuildOptions {
    fn default() -> Self {
        Self {
            output_dir: "dist".to_string(),
            assets_dir: "_maudit".to_string(),
            static_dir: "static".to_string(),
        }
    }
}

pub fn coronate(
    routes: Vec<&dyn FullPage>,
    content_sources: ContentSources,
    options: BuildOptions,
) -> Result<BuildOutput, Box<dyn std::error::Error>> {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async { build(routes, content_sources, options).await })
}

pub async fn build(
    routes: Vec<&dyn FullPage>,
    content_sources: ContentSources,
    options: BuildOptions,
) -> Result<BuildOutput, Box<dyn std::error::Error>> {
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
    let dist_dir = PathBuf::from_str(&options.output_dir)?;
    let assets_dir = PathBuf::from_str(&options.output_dir)?.join(&options.assets_dir);
    let tmp_dir = dist_dir.join("_tmp");
    let static_dir = PathBuf::from_str(&options.static_dir)?;

    let _ = fs::remove_dir_all(&dist_dir);
    fs::create_dir_all(&dist_dir)?;
    fs::create_dir_all(&assets_dir)?;

    print_title("generating pages");
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
    let mut build_pages_styles: FxHashSet<assets::Style> = FxHashSet::default();

    let dynamic_route_context = DynamicRouteContext {
        content: &content_sources,
    };

    for route in routes {
        let routes = route.routes(&dynamic_route_context);
        match routes.is_empty() {
            true => {
                let route_start = SystemTime::now();
                let mut page_assets = assets::PageAssets {
                    assets_dir: options.assets_dir.clone().into(),
                    ..Default::default()
                };

                let mut ctx = RouteContext {
                    params: page::RouteParams(FxHashMap::default()),
                    content: &content_sources,
                    assets: &mut page_assets,
                };

                let (file_path, mut file) = create_route_file(route, &ctx.params, &dist_dir)?;
                let result = route.render(&mut ctx);

                finish_route(
                    result,
                    &mut file,
                    &page_assets.included_styles,
                    &page_assets.included_scripts,
                    route.route_raw(),
                )?;

                let formatted_elasped_time =
                    format_elapsed_time(route_start.elapsed(), &route_format_options)?;
                info!(target: "build", "{} -> {} {}", route.route(&page::RouteParams(FxHashMap::default())), file_path.to_string_lossy().dimmed(), formatted_elasped_time);

                build_pages_assets.extend(page_assets.assets);
                build_pages_scripts.extend(page_assets.scripts);
                build_pages_styles.extend(page_assets.styles);

                build_metadata.pages.push(PageOutput {
                    route: route.route_raw().to_string(),
                    file_path: file_path.to_string_lossy().to_string(),
                    params: None,
                });
            }
            false => {
                info!(target: "build", "{}", route.route_raw().to_string().bold());

                for params in routes {
                    let mut pages_assets = assets::PageAssets {
                        assets_dir: options.assets_dir.clone().into(),
                        ..Default::default()
                    };
                    let route_start = SystemTime::now();
                    let mut ctx = RouteContext {
                        params,
                        content: &content_sources,
                        assets: &mut pages_assets,
                    };

                    let (file_path, mut file) = create_route_file(route, &ctx.params, &dist_dir)?;
                    let result = route.render(&mut ctx);

                    build_metadata.pages.push(PageOutput {
                        route: route.route_raw(),
                        file_path: file_path.to_string_lossy().to_string(),
                        params: Some(ctx.params.0),
                    });

                    finish_route(
                        result,
                        &mut file,
                        &pages_assets.included_styles,
                        &pages_assets.included_scripts,
                        route.route_raw(),
                    )?;

                    let formatted_elasped_time =
                        format_elapsed_time(route_start.elapsed(), &route_format_options)?;
                    info!(target: "build", "├─ {} {}", file_path.to_string_lossy().dimmed(), formatted_elasped_time);

                    build_pages_assets.extend(pages_assets.assets);
                    build_pages_scripts.extend(pages_assets.scripts);
                    build_pages_styles.extend(pages_assets.styles);
                }
            }
        }
    }

    let formatted_elasped_time =
        format_elapsed_time(pages_start.elapsed(), &section_format_options)?;
    info!(target: "build", "{}", format!("Pages generated in {}", formatted_elasped_time).bold());

    let assets_start = SystemTime::now();
    print_title("generating assets");

    build_pages_assets.iter().for_each(|asset| {
        asset.process(&assets_dir, &tmp_dir);

        // TODO: Add outputted assets to build_metadata, might need dedicated fs methods for this
    });

    let css_inputs = build_pages_styles
        .iter()
        .map(|style| {
            let processed_path = style.process(&assets_dir, &tmp_dir);

            InputItem {
                import: {
                    if let Some(processed_path) = processed_path {
                        processed_path
                    } else {
                        style.path().to_string_lossy().to_string()
                    }
                },
                ..Default::default()
            }
        })
        .collect::<Vec<InputItem>>();

    let bundler_inputs = build_pages_scripts
        .iter()
        .map(|script| InputItem {
            import: script.path().to_string_lossy().to_string(),
            ..Default::default()
        })
        .chain(css_inputs.into_iter())
        .collect::<Vec<InputItem>>();

    if !bundler_inputs.is_empty() {
        let mut bundler = Bundler::new(BundlerOptions {
            input: Some(bundler_inputs),
            minify: Some(true),
            dir: Some(assets_dir.to_string_lossy().to_string()),

            ..Default::default()
        });

        let _result = bundler.write().await.unwrap();

        // TODO: Add outputted chunks to build_metadata
    }

    let formatted_elasped_time =
        format_elapsed_time(assets_start.elapsed(), &section_format_options)?;
    info!(target: "build", "{}", format!("Assets generated in {}", formatted_elasped_time).bold());

    // Check if static directory exists
    if static_dir.exists() {
        let assets_start = SystemTime::now();
        print_title("copying assets");

        // Copy the static directory to the dist directory
        copy_recursively(&static_dir, &dist_dir, &mut build_metadata)?;

        let formatted_elasped_time =
            format_elapsed_time(assets_start.elapsed(), &FormatElapsedTimeOptions::default())?;
        info!(target: "build", "{}", format!("Assets copied in {}", formatted_elasped_time).bold());
    }

    // Remove temporary files
    let _ = remove_dir_all(&tmp_dir);

    let formatted_elasped_time =
        format_elapsed_time(build_start.elapsed(), &section_format_options)?;
    info!(target: "SKIP_FORMAT", "{}", "");
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
    dist_dir: &Path,
) -> Result<(PathBuf, File), Box<dyn std::error::Error>> {
    let file_path = dist_dir.join(route.file_path(params));

    // Create the parent directories if it doesn't exist
    if let Some(parent_dir) = file_path.parent() {
        fs::create_dir_all(parent_dir)?
    }

    // Create file
    let file = File::create(file_path.clone())?;

    Ok((file_path, file))
}

fn finish_route(
    render_result: RenderResult,
    file: &mut File,
    included_styles: &Vec<assets::Style>,
    included_scripts: &Vec<assets::Script>,
    route: String,
) -> Result<(), Box<dyn std::error::Error>> {
    match render_result {
        RenderResult::Html(html) => {
            let element_content_handlers = vec![
                // Add included scripts and styles to the head
                element!("head", |el| {
                    for style in included_styles {
                        el.append(
                            &format!(
                                "<link rel=\"stylesheet\" href=\"{}\">",
                                style.url().unwrap_or_else(|| panic!(
                                    "Failed to get URL for style: {:?}. This should not happen, please report this issue",
                                    style.path()
                                ))
                            ),
                            lol_html::html_content::ContentType::Html,
                        );
                    }

                    for script in included_scripts {
                        el.append(
                            &format!(
                                "<script src=\"{}\"></script>",
                                script.url().unwrap_or_else(|| panic!(
                                    "Failed to get URL for script: {:?}. This should not happen, please report this issue.",
                                    script.path()
                                ))
                            ),
                            lol_html::html_content::ContentType::Html,
                        );
                    }

                    Ok(())
                }),
            ];

            let output = rewrite_str(
                &html,
                RewriteStrSettings {
                    element_content_handlers,
                    ..RewriteStrSettings::new()
                },
            )?;

            file.write_all(output.as_bytes())?;
        }
        RenderResult::Text(text) => {
            if !included_scripts.is_empty() || !included_styles.is_empty() {
                Err(BuildError::InvalidRenderResult { route })?;
            }

            file.write_all(text.as_bytes())?;
        }
    }

    Ok(())
}

fn print_title(title: &str) {
    info!(target: "SKIP_FORMAT", "{}", "");
    info!(target: "SKIP_FORMAT", "{}", format!(" {} ", title).on_green().bold());
}
