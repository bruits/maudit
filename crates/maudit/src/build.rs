use core::panic;
use std::{
    env,
    fs::{self},
    io::{self},
    path::{Path, PathBuf},
    process::Command,
    sync::Arc,
    time::{Instant, SystemTime, UNIX_EPOCH},
};

use crate::{
    BuildOptions, BuildOutput,
    assets::{
        self, RouteAssets,
        image_cache::{IMAGE_CACHE_DIR, ImageCache},
    },
    build::images::process_image,
    content::{ContentSources, RouteContent},
    errors::BuildError,
    is_dev,
    logging::print_title,
    route::{DynamicRouteContext, FullRoute, PageContext, PageParams, RenderResult, RouteType},
};
use colored::{ColoredString, Colorize};
use log::{debug, info, trace, warn};
use oxc_sourcemap::SourceMap;
use rolldown::{
    Bundler, BundlerOptions, InputItem, ModuleType,
    plugin::{HookUsage, Plugin},
};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::assets::Asset;
use crate::logging::{FormatElapsedTimeOptions, format_elapsed_time};

use lol_html::{RewriteStrSettings, element, rewrite_str};
use rayon::prelude::*;

pub mod images;
pub mod metadata;
pub mod options;

#[derive(Debug)]
struct TailwindPlugin {
    tailwind_path: PathBuf,
    tailwind_entries: Vec<PathBuf>,
}

impl Plugin for TailwindPlugin {
    fn name(&self) -> std::borrow::Cow<'static, str> {
        "builtin:tailwind".into()
    }

    fn register_hook_usage(&self) -> rolldown::plugin::HookUsage {
        HookUsage::Transform
    }

    async fn transform(
        &self,
        _ctx: rolldown::plugin::SharedTransformPluginContext,
        args: &rolldown::plugin::HookTransformArgs<'_>,
    ) -> rolldown::plugin::HookTransformReturn {
        if *args.module_type != ModuleType::Css {
            return Ok(None);
        }

        if self
            .tailwind_entries
            .iter()
            .any(|entry| entry.canonicalize().unwrap().to_string_lossy() == args.id)
        {
            let start_tailwind = Instant::now();
            let mut command = Command::new(&self.tailwind_path);
            command.args(["--input", args.id]);

            // Add minify in production, source maps in development
            if !crate::is_dev() {
                command.arg("--minify");
            }
            if crate::is_dev() {
                command.arg("--map");
            }

            let tailwind_output = command.output()
                    .unwrap_or_else(|e| {
                        // TODO: Return a proper error instead of panicking
                        let args_str = if crate::is_dev() {
                            format!("['--input', '{}', '--map']", args.id)
                        } else {
                            format!("['--input', '{}', '--minify']", args.id)
                        };
                        panic!(
                            "Failed to execute Tailwind CSS command, is it installed and is the path to its binary correct?\nCommand: '{}', Args: {}. Error: {}",
                            &self.tailwind_path.display(),
                            args_str,
                            e
                        )
            });

            if !tailwind_output.status.success() {
                let stderr = String::from_utf8_lossy(&tailwind_output.stderr);
                let error_message = format!(
                    "Tailwind CSS process failed with status {}: {}",
                    tailwind_output.status, stderr
                );
                panic!("{}", error_message);
            }

            info!("Tailwind took {:?}", start_tailwind.elapsed());

            let output = String::from_utf8_lossy(&tailwind_output.stdout);
            let (code, map) = if let Some((code, map)) = output.split_once("/*# sourceMappingURL") {
                (code.to_string(), Some(map.to_string()))
            } else {
                (output.to_string(), None)
            };

            if let Some(map) = map {
                let source_map = SourceMap::from_json_string(&map).ok();

                return Ok(Some(rolldown::plugin::HookTransformOutput {
                    code: Some(code),
                    map: source_map,
                    ..Default::default()
                }));
            }

            return Ok(Some(rolldown::plugin::HookTransformOutput {
                code: Some(code),
                ..Default::default()
            }));
        }

        Ok(None)
    }
}

pub fn execute_build(
    routes: &[&dyn FullRoute],
    content_sources: &mut ContentSources,
    options: &BuildOptions,
    async_runtime: &tokio::runtime::Runtime,
) -> Result<BuildOutput, Box<dyn std::error::Error>> {
    async_runtime.block_on(async { build(routes, content_sources, options).await })
}

pub async fn build(
    routes: &[&dyn FullRoute],
    content_sources: &mut ContentSources,
    options: &BuildOptions,
) -> Result<BuildOutput, Box<dyn std::error::Error>> {
    let build_start = Instant::now();
    let mut build_metadata = BuildOutput::new(build_start);

    // Create a directory for the output
    trace!(target: "build", "Setting up required directories...");

    let old_dist_tmp_dir = if options.clean_output_dir {
        let duration = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        let num = (duration.as_secs() + duration.subsec_nanos() as u64) % 100000;
        let new_dir_for_old_dist = env::temp_dir().join(format!("maudit_old_dist_{}", num));
        let _ = fs::rename(&options.output_dir, &new_dir_for_old_dist);
        Some(new_dir_for_old_dist)
    } else {
        None
    };

    let should_clear_dist = options.clean_output_dir;
    let clean_up_handle = tokio::spawn(async move {
        if should_clear_dist {
            let _ = fs::remove_dir_all(old_dist_tmp_dir.unwrap());
        }
    });

    let page_assets_options = options.page_assets_options();

    info!(target: "build", "Output directory: {}", options.output_dir.display());

    let content_sources_start = Instant::now();
    print_title("initializing content sources");
    content_sources.sources_mut().iter_mut().for_each(|source| {
        let source_start = Instant::now();
        source.init();

        info!(target: "content", "{} initialized in {}", source.get_name(), format_elapsed_time(source_start.elapsed(), &FormatElapsedTimeOptions::default()));
    });

    info!(target: "content", "{}", format!("Content sources initialized in {}", format_elapsed_time(
        content_sources_start.elapsed(),
        &FormatElapsedTimeOptions::default(),
    )).bold());

    print_title("generating pages");
    let pages_start = Instant::now();

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

    let mut build_pages_images: FxHashSet<assets::Image> = FxHashSet::default();
    let mut build_pages_scripts: FxHashSet<assets::Script> = FxHashSet::default();
    let mut build_pages_styles: FxHashSet<assets::Style> = FxHashSet::default();

    // Parallel processing of routes
    let (page_count, all_images, all_scripts, all_styles, all_metadata) = routes
        .par_iter()
        .map(|route| {
            match route.route_type() {
                RouteType::Static => {
                    let route_start = Instant::now();

                    let content = RouteContent::new(content_sources);
                    let mut page_assets = RouteAssets::new(&page_assets_options);

                    let params = PageParams::default();
                    let url = route.url(&params);

                    let result = route.build(&mut PageContext::from_static_route(
                        &content,
                        &mut page_assets,
                        &url,
                    )).expect("Failed to build static route");

                    let file_path = route.file_path(&params, &options.output_dir);

                    write_route_file(&result, &file_path).expect("Failed to write route file");

                    info!(target: "pages", "{} -> {} {}", url, file_path.to_string_lossy().dimmed(), format_elapsed_time(route_start.elapsed(), &route_format_options));

                    let metadata_entry = (
                        route.route_raw().to_string(),
                        file_path.to_string_lossy().to_string(),
                        None,
                    );

                    (
                        1,
                        page_assets.images,
                        page_assets.scripts,
                        page_assets.styles,
                        vec![metadata_entry],
                    )
                }
                RouteType::Dynamic => {
                    let content = RouteContent::new(content_sources);

                    // Create a temporary assets context for getting pages
                    let mut temp_page_assets = RouteAssets::new(&page_assets_options);
                    let pages = route.get_pages(&mut DynamicRouteContext {
                        content: &content,
                        assets: &mut temp_page_assets,
                    });

                    if pages.is_empty() {
                        warn!(target: "build", "{} is a dynamic route, but its implementation of Route::pages returned an empty Vec. No pages will be generated for this route.", route.route_raw().to_string().bold());
                        return (
                            0,
                            FxHashSet::default(),
                            FxHashSet::default(),
                            FxHashSet::default(),
                            vec![],
                        );
                    } else {
                        info!(target: "build", "{}", route.route_raw().to_string().bold());
                    }

                    let page_count = pages.len();

                    let page_results: Vec<_> = pages
                        .par_iter()
                        .map(|page| {
                            let route_start = Instant::now();

                            // Each parallel task gets its own assets
                            let mut individual_page_assets = RouteAssets::new(&page_assets_options);
                            let content = RouteContent::new(content_sources);

                            let url = route.url(&page.0);

                            let content = route.build(&mut PageContext::from_dynamic_route(
                                page,
                                &content,
                                &mut individual_page_assets,
                                &url,
                            )).expect("Failed to build dynamic route");

                            let file_path = route.file_path(&page.0, &options.output_dir);

                            write_route_file(&content, &file_path).expect("Failed to write route file");

                            info!(target: "pages", "├─ {} {}", file_path.to_string_lossy().dimmed(), format_elapsed_time(route_start.elapsed(), &route_format_options));

                            let metadata_entry = (
                                route.route_raw().to_string(),
                                file_path.to_string_lossy().to_string(),
                                Some(page.0.0.clone()),
                            );

                            (metadata_entry, individual_page_assets)
                        })
                        .collect();

                    // Collect metadata and merge all assets
                    let mut metadata_entries = Vec::new();
                    let mut merged_images = FxHashSet::default();
                    let mut merged_scripts = FxHashSet::default();
                    let mut merged_styles = FxHashSet::default();

                    // Add assets from the get_pages call
                    merged_images.extend(temp_page_assets.images);
                    merged_scripts.extend(temp_page_assets.scripts);
                    merged_styles.extend(temp_page_assets.styles);

                    for (metadata_entry, assets) in page_results {
                        metadata_entries.push(metadata_entry);
                        merged_images.extend(assets.images);
                        merged_scripts.extend(assets.scripts);
                        merged_styles.extend(assets.styles);
                    }

                    (
                        page_count,
                        merged_images,
                        merged_scripts,
                        merged_styles,
                        metadata_entries,
                    )
                }
            }
        })
        .fold(
            || (0, FxHashSet::default(), FxHashSet::default(), FxHashSet::default(), Vec::new()),
            |mut acc, item| {
                acc.0 += item.0;
                acc.1.extend(item.1);
                acc.2.extend(item.2);
                acc.3.extend(item.3);
                acc.4.extend(item.4);
                acc
            },
        )
        .reduce(
            || (0, FxHashSet::default(), FxHashSet::default(), FxHashSet::default(), Vec::new()),
            |mut acc, item| {
                acc.0 += item.0;
                acc.1.extend(item.1);
                acc.2.extend(item.2);
                acc.3.extend(item.3);
                acc.4.extend(item.4);
                acc
            },
        );

    // Add metadata entries to build_metadata
    for (route_raw, file_path, params) in all_metadata {
        build_metadata.add_page(route_raw, file_path, params);
    }

    // Collect all assets
    build_pages_images.extend(all_images);
    build_pages_scripts.extend(all_scripts);
    build_pages_styles.extend(all_styles);

    info!(target: "pages", "{}", format!("generated {} pages in {}", page_count,  format_elapsed_time(pages_start.elapsed(), &section_format_options)).bold());

    if (!build_pages_images.is_empty())
        || !build_pages_styles.is_empty()
        || !build_pages_scripts.is_empty()
    {
        fs::create_dir_all(&page_assets_options.output_assets_dir)?;
    }

    if !build_pages_styles.is_empty() || !build_pages_scripts.is_empty() {
        let assets_start = Instant::now();
        print_title("generating assets");

        let css_inputs = build_pages_styles
            .iter()
            .map(|style| InputItem {
                name: Some(
                    style
                        .filename()
                        .with_extension("")
                        .to_string_lossy()
                        .to_string(),
                ),
                import: { style.path().to_string_lossy().to_string() },
            })
            .collect::<Vec<InputItem>>();

        let bundler_inputs = build_pages_scripts
            .iter()
            .map(|script| InputItem {
                import: script.path().to_string_lossy().to_string(),
                name: Some(
                    script
                        .filename()
                        .with_extension("")
                        .to_string_lossy()
                        .to_string(),
                ),
            })
            .chain(css_inputs.into_iter())
            .collect::<Vec<InputItem>>();

        if !bundler_inputs.is_empty() {
            let mut module_types_hashmap = FxHashMap::default();
            module_types_hashmap.insert("woff".to_string(), ModuleType::Asset);
            module_types_hashmap.insert("woff2".to_string(), ModuleType::Asset);

            let mut bundler = Bundler::with_plugins(
                BundlerOptions {
                    input: Some(bundler_inputs),
                    minify: Some(rolldown::RawMinifyOptions::Bool(!is_dev())),
                    dir: Some(
                        page_assets_options
                            .output_assets_dir
                            .to_string_lossy()
                            .to_string(),
                    ),
                    module_types: Some(module_types_hashmap),
                    ..Default::default()
                },
                vec![Arc::new(TailwindPlugin {
                    tailwind_path: options.assets.tailwind_binary_path.clone(),
                    tailwind_entries: build_pages_styles
                        .iter()
                        .filter_map(|style| {
                            if style.tailwind {
                                Some(style.path().clone())
                            } else {
                                None
                            }
                        })
                        .collect::<Vec<PathBuf>>(),
                })],
            );

            let _result = bundler.write().await.unwrap();

            // TODO: Add outputted chunks to build_metadata
        }

        info!(target: "build", "{}", format!("Assets generated in {}", format_elapsed_time(assets_start.elapsed(), &section_format_options)).bold());
    }

    if !build_pages_images.is_empty() {
        print_title("processing images");

        let _ = fs::create_dir_all(IMAGE_CACHE_DIR);

        let start_time = Instant::now();
        build_pages_images.par_iter().for_each(|image| {
            let start_process = Instant::now();
            let dest_path: &PathBuf = image.build_path();

            if let Some(image_options) = &image.options {
                let final_filename = image.filename();

                // Check cache for transformed images
                if let Some(cached_path) = ImageCache::get_transformed_image(final_filename) {
                    // Copy from cache instead of processing
                    if fs::copy(&cached_path, dest_path).is_ok() {
                        info!(target: "assets", "{} -> {} (from cache) {}", image.path().to_string_lossy(), dest_path.to_string_lossy().dimmed(), format_elapsed_time(start_process.elapsed(), &route_format_options).dimmed());
                        return;
                    }
                }

                // Generate cache path for transformed image
                let cache_path = ImageCache::generate_cache_path(final_filename);

                // Process image directly to cache
                process_image(image, &cache_path, image_options);

                // Copy from cache to destination
                if fs::copy(&cache_path, dest_path).is_ok() {
                    // Cache the processed image path
                    ImageCache::cache_transformed_image(final_filename, cache_path);
                } else {
                    debug!("Failed to copy from cache {} to dest {}", cache_path.display(), dest_path.display());
                }
            } else if !dest_path.exists() {
                // TODO: Check if copying should be done in this parallel iterator, I/O doesn't benefit from parallelism so having those tasks here might just be slowing processing
                fs::copy(image.path(), dest_path).unwrap_or_else(|e| {
                    panic!(
                        "Failed to copy image from {} to {}: {}",
                        image.path().to_string_lossy(),
                        dest_path.to_string_lossy(),
                        e
                    )
                });
            }
            info!(target: "assets", "{} -> {} {}", image.path().to_string_lossy(), dest_path.to_string_lossy().dimmed(), format_elapsed_time(start_process.elapsed(), &route_format_options).dimmed());
        });

        info!(target: "assets", "{}", format!("Images processed in {}", format_elapsed_time(start_time.elapsed(), &section_format_options)).bold());
    }

    // Check if static directory exists
    if options.static_dir.exists() {
        let assets_start = Instant::now();
        print_title("copying assets");

        // Copy the static directory to the dist directory
        copy_recursively(
            &options.static_dir,
            &options.output_dir,
            &mut build_metadata,
        )?;

        info!(target: "build", "{}", format!("Assets copied in {}", format_elapsed_time(assets_start.elapsed(), &FormatElapsedTimeOptions::default())).bold());
    }

    info!(target: "SKIP_FORMAT", "{}", "");
    info!(target: "build", "{}", format!("Build completed in {}", format_elapsed_time(build_start.elapsed(), &section_format_options)).bold());

    clean_up_handle.await.unwrap();

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

fn write_route_file(content: &[u8], file_path: &PathBuf) -> Result<(), io::Error> {
    // Create the parent directories if it doesn't exist
    if let Some(parent_dir) = file_path.parent() {
        fs::create_dir_all(parent_dir)?
    }

    fs::write(file_path, content)?;

    Ok(())
}

pub fn finish_route(
    render_result: RenderResult,
    page_assets: &assets::RouteAssets,
    route: String,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    match render_result {
        RenderResult::Text(html) => {
            let included_styles: Vec<_> = page_assets.included_styles().collect();
            let included_scripts: Vec<_> = page_assets.included_scripts().collect();

            if included_scripts.is_empty() && included_styles.is_empty() {
                return Ok(html.into_bytes());
            }

            let element_content_handlers = vec![
                // Add included scripts and styles to the head
                element!("head", |el| {
                    for style in &included_styles {
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

                    for script in &included_scripts {
                        el.append(
                            &format!(
                                "<script src=\"{}\" type=\"module\"></script>",
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

            Ok(output.into_bytes())
        }
        RenderResult::Raw(content) => {
            let included_styles: Vec<_> = page_assets.included_styles().collect();
            let included_scripts: Vec<_> = page_assets.included_scripts().collect();

            if !included_scripts.is_empty() || !included_styles.is_empty() {
                Err(BuildError::InvalidRenderResult { route })?;
            }

            Ok(content)
        }
    }
}
