use std::{
    fs::{self, remove_dir_all, File},
    io::{self, Write},
    path::{Path, PathBuf},
    str::FromStr,
    time::SystemTime,
};

use crate::{
    assets,
    content::{Content, ContentSources},
    errors::BuildError,
    logging::print_title,
    page::{DynamicRouteContext, FullPage, RenderResult, RouteContext, RouteParams, RouteType},
    route::{
        extract_params_from_raw_route, get_route_file_path, get_route_type_from_route_params,
        get_route_url, ParameterDef,
    },
    BuildOptions, BuildOutput,
};
use colored::{ColoredString, Colorize};
use log::{info, trace};
use rolldown::{Bundler, BundlerOptions, InputItem};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::assets::Asset;
use crate::logging::{format_elapsed_time, FormatElapsedTimeOptions};

use lol_html::{element, rewrite_str, RewriteStrSettings};

pub mod metadata;
pub mod options;

pub fn execute_build(
    routes: &[&dyn FullPage],
    content_sources: &mut ContentSources,
    options: &BuildOptions,
    async_runtime: &tokio::runtime::Runtime,
) -> Result<BuildOutput, Box<dyn std::error::Error>> {
    async_runtime.block_on(async { build(routes, content_sources, options).await })
}

pub async fn build(
    routes: &[&dyn FullPage],
    content_sources: &mut ContentSources,
    options: &BuildOptions,
) -> Result<BuildOutput, Box<dyn std::error::Error>> {
    let build_start = SystemTime::now();
    let mut build_metadata = BuildOutput::new(build_start);

    // Create a directory for the output
    trace!(target: "build", "Setting up required directories...");
    let dist_dir = PathBuf::from_str(&options.output_dir)?;
    let assets_dir = PathBuf::from_str(&options.output_dir)?.join(&options.assets_dir);
    let tmp_dir = dist_dir.join("_tmp");
    let static_dir = PathBuf::from_str(&options.static_dir)?;

    let _ = fs::remove_dir_all(&dist_dir);
    fs::create_dir_all(&dist_dir)?;
    fs::create_dir_all(&assets_dir)?;

    info!(target: "build", "Output directory: {}", dist_dir.to_string_lossy());

    let content_sources_start = SystemTime::now();
    print_title("initializing content sources");
    content_sources.0.iter_mut().for_each(|source| {
        let source_start = SystemTime::now();
        source.init();

        info!(target: "build", "{}", format!("{} initialized in {}", source.get_name(), format_elapsed_time(source_start.elapsed(), &FormatElapsedTimeOptions::default()).unwrap()));
    });

    info!(target: "build", "{}", format!("Content sources initialized in {}", format_elapsed_time(
        content_sources_start.elapsed(),
        &FormatElapsedTimeOptions::default(),
    ).unwrap()).bold());

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

    let mut build_pages_assets: FxHashSet<Box<dyn Asset>> = FxHashSet::default();
    let mut build_pages_scripts: FxHashSet<assets::Script> = FxHashSet::default();
    let mut build_pages_styles: FxHashSet<assets::Style> = FxHashSet::default();

    let mut page_count = 0;
    for route in routes {
        let params_def = extract_params_from_raw_route(&route.route_raw());
        let route_type = get_route_type_from_route_params(&params_def);
        match route_type {
            RouteType::Static => {
                let route_start = SystemTime::now();
                let mut page_assets = assets::PageAssets {
                    assets_dir: options.assets_dir.clone().into(),
                    tailwind_path: options.tailwind_binary_path.clone().into(),
                    ..Default::default()
                };

                let params = RouteParams(FxHashMap::default());

                let mut content = Content::new(&content_sources.0);
                let mut ctx = RouteContext {
                    raw_params: &params,
                    content: &mut content,
                    assets: &mut page_assets,
                    current_url: String::new(), // TODO
                };

                let (file_path, mut file) =
                    create_route_file(*route, &params_def, ctx.raw_params, &dist_dir)?;
                let result = route.render_internal(&mut ctx);

                finish_route(
                    result,
                    &mut file,
                    &page_assets.included_styles,
                    &page_assets.included_scripts,
                    route.route_raw(),
                )?;

                info!(target: "build", "{} -> {} {}", get_route_url(&route.route_raw(), &params_def, &params), file_path.to_string_lossy().dimmed(), format_elapsed_time(route_start.elapsed(), &route_format_options).unwrap());

                build_pages_assets.extend(page_assets.assets);
                build_pages_scripts.extend(page_assets.scripts);
                build_pages_styles.extend(page_assets.styles);

                build_metadata.add_page(
                    route.route_raw().to_string(),
                    file_path.to_string_lossy().to_string(),
                    None,
                );

                page_count += 1;
            }
            RouteType::Dynamic => {
                let mut dynamic_content = Content::new(&content_sources.0);
                let mut dynamic_route_context = DynamicRouteContext {
                    content: &mut dynamic_content,
                };

                let routes = route.routes_internal(&mut dynamic_route_context);

                if routes.is_empty() {
                    info!(target: "build", "{} is a dynamic route, but its implementation of Page::routes returned an empty Vec. No pages will be generated for this route.", route.route_raw().to_string().bold());
                    continue;
                } else {
                    info!(target: "build", "{}", route.route_raw().to_string().bold());
                }

                for params in routes {
                    let mut pages_assets = assets::PageAssets {
                        assets_dir: options.assets_dir.clone().into(),
                        tailwind_path: options.tailwind_binary_path.clone().into(),
                        ..Default::default()
                    };
                    let route_start = SystemTime::now();
                    let mut content = Content::new(&content_sources.0);
                    let mut ctx = RouteContext {
                        raw_params: &params,
                        content: &mut content,
                        assets: &mut pages_assets,
                        current_url: String::new(), // TODO
                    };

                    let (file_path, mut file) =
                        create_route_file(*route, &params_def, ctx.raw_params, &dist_dir)?;

                    let result = route.render_internal(&mut ctx);

                    build_metadata.add_page(
                        route.route_raw().to_string(),
                        file_path.to_string_lossy().to_string(),
                        Some(params.0),
                    );

                    finish_route(
                        result,
                        &mut file,
                        &pages_assets.included_styles,
                        &pages_assets.included_scripts,
                        route.route_raw(),
                    )?;

                    info!(target: "build", "├─ {} {}", file_path.to_string_lossy().dimmed(), format_elapsed_time(route_start.elapsed(), &route_format_options).unwrap());

                    build_pages_assets.extend(pages_assets.assets);
                    build_pages_scripts.extend(pages_assets.scripts);
                    build_pages_styles.extend(pages_assets.styles);

                    page_count += 1;
                }
            }
        }
    }

    info!(target: "build", "{}", format!("generated {} pages in {}", page_count,  format_elapsed_time(pages_start.elapsed(), &section_format_options).unwrap()).bold());

    if !build_pages_assets.is_empty()
        || !build_pages_styles.is_empty()
        || !build_pages_scripts.is_empty()
    {
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

        info!(target: "build", "{}", format!("Assets generated in {}", format_elapsed_time(assets_start.elapsed(), &section_format_options).unwrap()).bold());
    }

    // Check if static directory exists
    if static_dir.exists() {
        let assets_start = SystemTime::now();
        print_title("copying assets");

        // Copy the static directory to the dist directory
        copy_recursively(&static_dir, &dist_dir, &mut build_metadata)?;

        info!(target: "build", "{}", format!("Assets copied in {}", format_elapsed_time(assets_start.elapsed(), &FormatElapsedTimeOptions::default()).unwrap()).bold());
    }

    // Remove temporary files
    let _ = remove_dir_all(&tmp_dir);

    info!(target: "SKIP_FORMAT", "{}", "");
    info!(target: "build", "{}", format!("Build completed in {}", format_elapsed_time(build_start.elapsed(), &section_format_options).unwrap()).bold());

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

            build_metadata.add_static_file(
                destination
                    .as_ref()
                    .join(entry.file_name())
                    .to_string_lossy()
                    .to_string(),
                entry.path().to_string_lossy().to_string(),
            );
        }
    }
    Ok(())
}

fn create_route_file(
    route: &dyn FullPage,
    params_def: &Vec<ParameterDef>,
    params: &RouteParams,
    dist_dir: &Path,
) -> Result<(PathBuf, File), Box<dyn std::error::Error>> {
    let file_path = dist_dir.join(get_route_file_path(
        &route.route_raw(),
        params_def,
        params,
        route.is_endpoint(),
    ));

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
    included_styles: &[assets::Style],
    included_scripts: &[assets::Script],
    route: String,
) -> Result<(), Box<dyn std::error::Error>> {
    match render_result {
        RenderResult::Text(html) => {
            if included_scripts.is_empty() && included_styles.is_empty() {
                file.write_all(html.as_bytes())?;
                return Ok(());
            }

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
        RenderResult::Raw(content) => {
            if !included_scripts.is_empty() || !included_styles.is_empty() {
                Err(BuildError::InvalidRenderResult { route })?;
            }

            file.write_all(&content)?;
        }
    }

    Ok(())
}
