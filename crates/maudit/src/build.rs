use core::panic;
use std::{
    env,
    fs::{self},
    io::{self},
    path::{Path, PathBuf},
    sync::Arc,
    time::{Instant, SystemTime, UNIX_EPOCH},
};

use crate::{
    BuildOptions, BuildOutput,
    assets::{
        self, HashAssetType, HashConfig, PrefetchPlugin, RouteAssets, Script, TailwindPlugin,
        calculate_hash, image_cache::ImageCache, prefetch,
    },
    build::{images::process_image, options::PrefetchStrategy},
    content::ContentSources,
    is_dev,
    logging::print_title,
    route::{CachedRoute, DynamicRouteContext, FullRoute, InternalRoute, PageContext, PageParams},
    routing::extract_params_from_raw_route,
    sitemap::{SitemapEntry, generate_sitemap},
};
use colored::{ColoredString, Colorize};
use log::{debug, info, trace, warn};
use pathdiff::diff_paths;
use rolldown::{Bundler, BundlerOptions, InputItem, ModuleType};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::assets::Asset;
use crate::logging::{FormatElapsedTimeOptions, format_elapsed_time};
use rayon::prelude::*;

pub mod images;
pub mod metadata;
pub mod options;

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

    let clean_up_handle = if options.clean_output_dir {
        let old_dist_tmp_dir = {
            let duration = SystemTime::now().duration_since(UNIX_EPOCH)?;
            let num = (duration.as_secs() + duration.subsec_nanos() as u64) % 100000;
            let new_dir_for_old_dist = env::temp_dir().join(format!("maudit_old_dist_{}", num));
            let _ = fs::rename(&options.output_dir, &new_dir_for_old_dist);
            new_dir_for_old_dist
        };

        Some(tokio::spawn(async {
            let _ = fs::remove_dir_all(old_dist_tmp_dir);
        }))
    } else {
        None
    };

    // Create the image cache early so it can be shared across routes
    let image_cache = ImageCache::with_cache_dir(&options.assets.image_cache_dir);
    let _ = fs::create_dir_all(image_cache.get_cache_dir());

    // Create route_assets_options with the image cache
    let route_assets_options = options.route_assets_options();

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

    // This is okay, build_pages_images Hash function does not use mutable data
    #[allow(clippy::mutable_key_type)]
    let mut build_pages_images: FxHashSet<assets::Image> = FxHashSet::default();
    let mut build_pages_scripts: FxHashSet<assets::Script> = FxHashSet::default();
    let mut build_pages_styles: FxHashSet<assets::Style> = FxHashSet::default();

    let mut sitemap_entries: Vec<SitemapEntry> = Vec::new();
    let mut page_count = 0;

    // Normalize base_url once to avoid repeated trimming
    let normalized_base_url = options
        .base_url
        .as_ref()
        .map(|url| url.trim_end_matches('/'));

    let mut default_scripts = vec![];

    let prefetch_path = match options.prefetch.strategy {
        PrefetchStrategy::None => None,
        PrefetchStrategy::Hover => Some(PathBuf::from(prefetch::PREFETCH_HOVER_PATH)),
        PrefetchStrategy::Tap => Some(PathBuf::from(prefetch::PREFETCH_TAP_PATH)),
        PrefetchStrategy::Viewport => Some(PathBuf::from(prefetch::PREFETCH_VIEWPORT_PATH)),
    };

    if let Some(prefetch_path) = prefetch_path {
        let prefetch_script = Script::new(
            prefetch_path.clone(),
            true,
            calculate_hash(
                &prefetch_path,
                Some(&HashConfig {
                    asset_type: HashAssetType::Script,
                    hashing_strategy: &options.assets.hashing_strategy,
                }),
            )?,
            &route_assets_options,
        );
        default_scripts.push(prefetch_script);
    }

    // This is fully serial. It is somewhat trivial to make it parallel, but it currently isn't because every time I've tried to
    // (uncommited, #25, #41, #46) it either made no difference or was slower. The overhead of parallelism is just too high for
    // how fast most sites build. Ideally, it'd be configurable and default to serial, but I haven't found an ergonomic way to do that yet.
    // If you manage to make it parallel and it actually improves performance, please open a PR!
    for route in routes {
        let route_start = Instant::now();
        let cached_route = CachedRoute::new(*route);
        let base_path = route.route_raw();
        let variants = cached_route.variants();

        trace!(target: "build", "Processing route: base='{}', variants={}", base_path.as_deref().unwrap_or(""), variants.len());

        let has_base_route = base_path.is_some();

        if !has_base_route && !variants.is_empty() {
            info!(target: "pages", "(variants only)");
        }

        // Handle base route
        if let Some(ref base_path) = base_path {
            let base_params = extract_params_from_raw_route(base_path);

            // Static base route
            if base_params.is_empty() {
                let mut route_assets = RouteAssets::with_default_assets(
                    &route_assets_options,
                    Some(image_cache.clone()),
                    default_scripts.clone(),
                    vec![],
                );

                let params = PageParams::default();
                let url = cached_route.url(&params);

                let result = route.build(&mut PageContext::from_static_route(
                    content_sources,
                    &mut route_assets,
                    &url,
                    &options.base_url,
                    None,
                ))?;

                let file_path = cached_route.file_path(&params, &options.output_dir);

                write_route_file(&result, &file_path)?;

                info!(target: "pages", "{} -> {} {}", url, file_path.to_string_lossy().dimmed(), format_elapsed_time(route_start.elapsed(), &route_format_options));

                build_pages_images.extend(route_assets.images);
                build_pages_scripts.extend(route_assets.scripts);
                build_pages_styles.extend(route_assets.styles);

                build_metadata.add_page(
                    base_path.clone(),
                    file_path.to_string_lossy().to_string(),
                    None,
                );

                add_sitemap_entry(
                    &mut sitemap_entries,
                    normalized_base_url,
                    &url,
                    base_path,
                    &route.sitemap_metadata(),
                    &options.sitemap,
                );

                page_count += 1;
            } else {
                // Dynamic base route
                let mut route_assets = RouteAssets::with_default_assets(
                    &route_assets_options,
                    Some(image_cache.clone()),
                    default_scripts.clone(),
                    vec![],
                );
                let pages = route.get_pages(&mut DynamicRouteContext {
                    content: content_sources,
                    assets: &mut route_assets,
                    variant: None,
                });

                if pages.is_empty() {
                    warn!(target: "build", "{} is a dynamic route, but its implementation of Route::pages returned an empty Vec. No pages will be generated for this route.", base_path.bold());
                    continue;
                } else {
                    // Log the pattern first
                    info!(target: "pages", "{}", base_path);

                    // Build all pages for this route
                    for page in pages {
                        let page_start = Instant::now();
                        let url = cached_route.url(&page.0);
                        let file_path = cached_route.file_path(&page.0, &options.output_dir);

                        let content = route.build(&mut PageContext::from_dynamic_route(
                            &page,
                            content_sources,
                            &mut route_assets,
                            &url,
                            &options.base_url,
                            None,
                        ))?;

                        write_route_file(&content, &file_path)?;

                        info!(target: "pages", "├─ {} {}", file_path.to_string_lossy().dimmed(), format_elapsed_time(page_start.elapsed(), &route_format_options));

                        build_metadata.add_page(
                            base_path.clone(),
                            file_path.to_string_lossy().to_string(),
                            Some(page.0.0.clone()),
                        );

                        add_sitemap_entry(
                            &mut sitemap_entries,
                            normalized_base_url,
                            &url,
                            base_path,
                            &route.sitemap_metadata(),
                            &options.sitemap,
                        );

                        page_count += 1;
                    }
                }

                build_pages_images.extend(route_assets.images);
                build_pages_scripts.extend(route_assets.scripts);
                build_pages_styles.extend(route_assets.styles);
            }
        }

        // Handle variants
        for (variant_id, variant_path) in variants {
            let variant_start = Instant::now();
            let variant_params = extract_params_from_raw_route(&variant_path);

            if variant_params.is_empty() {
                // Static variant
                let mut route_assets = RouteAssets::with_default_assets(
                    &route_assets_options,
                    Some(image_cache.clone()),
                    default_scripts.clone(),
                    vec![],
                );

                let params = PageParams::default();
                let url = cached_route.variant_url(&params, &variant_id)?;
                let file_path =
                    cached_route.variant_file_path(&params, &options.output_dir, &variant_id)?;

                let result = route.build(&mut PageContext::from_static_route(
                    content_sources,
                    &mut route_assets,
                    &url,
                    &options.base_url,
                    Some(variant_id.clone()),
                ))?;

                write_route_file(&result, &file_path)?;

                info!(target: "pages", "├─ {} {}", file_path.to_string_lossy().dimmed(), format_elapsed_time(variant_start.elapsed(), &route_format_options));

                build_pages_images.extend(route_assets.images);
                build_pages_scripts.extend(route_assets.scripts);
                build_pages_styles.extend(route_assets.styles);

                build_metadata.add_page(
                    variant_path.clone(),
                    file_path.to_string_lossy().to_string(),
                    None,
                );

                add_sitemap_entry(
                    &mut sitemap_entries,
                    normalized_base_url,
                    &url,
                    &variant_path,
                    &route.sitemap_metadata(),
                    &options.sitemap,
                );

                page_count += 1;
            } else {
                // Dynamic variant
                let mut route_assets = RouteAssets::with_default_assets(
                    &route_assets_options,
                    Some(image_cache.clone()),
                    default_scripts.clone(),
                    vec![],
                );
                let pages = route.get_pages(&mut DynamicRouteContext {
                    content: content_sources,
                    assets: &mut route_assets,
                    variant: Some(&variant_id),
                });

                if pages.is_empty() {
                    warn!(target: "build", "Variant {} has dynamic parameters but Route::pages returned an empty Vec.", variant_id.bold());
                } else {
                    // Log the variant pattern first
                    info!(target: "pages", "├─ {}", variant_path);

                    // Build all pages for this variant group
                    for page in pages {
                        let variant_page_start = Instant::now();
                        let url = cached_route.variant_url(&page.0, &variant_id)?;
                        let file_path = cached_route.variant_file_path(
                            &page.0,
                            &options.output_dir,
                            &variant_id,
                        )?;

                        let content = route.build(&mut PageContext::from_dynamic_route(
                            &page,
                            content_sources,
                            &mut route_assets,
                            &url,
                            &options.base_url,
                            Some(variant_id.clone()),
                        ))?;

                        write_route_file(&content, &file_path)?;

                        info!(target: "pages", "│  ├─ {} {}", file_path.to_string_lossy().dimmed(), format_elapsed_time(variant_page_start.elapsed(), &route_format_options));

                        build_metadata.add_page(
                            variant_path.clone(),
                            file_path.to_string_lossy().to_string(),
                            Some(page.0.0.clone()),
                        );

                        add_sitemap_entry(
                            &mut sitemap_entries,
                            normalized_base_url,
                            &url,
                            &variant_path,
                            &route.sitemap_metadata(),
                            &options.sitemap,
                        );

                        page_count += 1;
                    }
                }

                build_pages_images.extend(route_assets.images);
                build_pages_scripts.extend(route_assets.scripts);
                build_pages_styles.extend(route_assets.styles);
            }
        }
    }

    info!(target: "pages", "{}", format!("generated {} pages in {}", page_count,  format_elapsed_time(pages_start.elapsed(), &section_format_options)).bold());

    if (!build_pages_images.is_empty())
        || !build_pages_styles.is_empty()
        || !build_pages_scripts.is_empty()
    {
        fs::create_dir_all(&route_assets_options.output_assets_dir)?;
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

        debug!(
            target: "bundling",
            "Bundler inputs: {:?}",
            bundler_inputs
                .iter()
                .map(|input| input.import.clone())
                .collect::<Vec<String>>()
        );

        if !bundler_inputs.is_empty() {
            let mut module_types_hashmap = FxHashMap::default();
            module_types_hashmap.insert("woff".to_string(), ModuleType::Asset);
            module_types_hashmap.insert("woff2".to_string(), ModuleType::Asset);

            let mut bundler = Bundler::with_plugins(
                BundlerOptions {
                    input: Some(bundler_inputs),
                    minify: Some(rolldown::RawMinifyOptions::Bool(!is_dev())),
                    dir: Some(
                        route_assets_options
                            .output_assets_dir
                            .to_string_lossy()
                            .to_string(),
                    ),
                    module_types: Some(module_types_hashmap),
                    ..Default::default()
                },
                vec![
                    Arc::new(TailwindPlugin {
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
                    }),
                    Arc::new(PrefetchPlugin {}),
                ],
            )?;

            let _result = bundler.write().await?;

            // TODO: Add outputted chunks to build_metadata
        }

        info!(target: "build", "{}", format!("Assets generated in {}", format_elapsed_time(assets_start.elapsed(), &section_format_options)).bold());
    }

    if !build_pages_images.is_empty() {
        print_title("processing images");

        let start_time = Instant::now();
        build_pages_images.par_iter().for_each(|image| {
            let start_process = Instant::now();
            let dest_path: &PathBuf = image.build_path();

            let image_cwd_relative = diff_paths(image.path(), env::current_dir().unwrap())
                .unwrap_or_else(|| image.path().to_path_buf());

            if let Some(image_options) = &image.options {
                let final_filename = image.filename();

                // Check cache for transformed images
                let cached_path = image_cache.get_transformed_image(final_filename);

                if let Some(cached_path) = cached_path {
                    // Copy from cache instead of processing
                    if fs::copy(&cached_path, dest_path).is_ok() {
                        info!(target: "assets", "{} -> {} (from cache) {}", image_cwd_relative.to_string_lossy(), dest_path.to_string_lossy().dimmed(), format_elapsed_time(start_process.elapsed(), &route_format_options).dimmed());
                        return;
                    }
                }

                // Generate cache path for transformed image
                let cache_path = image_cache.generate_cache_path(final_filename);

                // Process image directly to cache
                process_image(image, &cache_path, image_options);

                // Copy from cache to destination
                if fs::copy(&cache_path, dest_path).is_ok() {
                    // Cache the processed image path
                    image_cache.cache_transformed_image(final_filename, cache_path);
                } else {
                    debug!("Failed to copy from cache {} to dest {}", cache_path.display(), dest_path.display());
                }
            } else if !dest_path.exists() {
                fs::copy(image.path(), dest_path).unwrap_or_else(|e| {
                    panic!(
                        "Failed to copy image from {} to {}: {}",
                        image.path().to_string_lossy(),
                        dest_path.to_string_lossy(),
                        e
                    )
                });
            }
            info!(target: "assets", "{} -> {} {}", image_cwd_relative.to_string_lossy(), dest_path.to_string_lossy().dimmed(), format_elapsed_time(start_process.elapsed(), &route_format_options).dimmed());
        });

        info!(target: "assets", "{}", format!("Images processed in {}", format_elapsed_time(start_time.elapsed(), &section_format_options)).bold());
    }

    // Check if static directory exists
    if options.static_dir.exists() {
        let assets_start = Instant::now();
        print_title("copying assets");

        copy_recursively(
            &options.static_dir,
            &options.output_dir,
            &mut build_metadata,
        )?;

        info!(target: "build", "{}", format!("Assets copied in {}", format_elapsed_time(assets_start.elapsed(), &FormatElapsedTimeOptions::default())).bold());
    }

    // Generate sitemap
    if options.sitemap.enabled {
        if let Some(base_url) = normalized_base_url {
            let sitemap_start = Instant::now();
            print_title("generating sitemap");

            generate_sitemap(
                sitemap_entries,
                base_url,
                &options.output_dir,
                &options.sitemap,
            )?;

            info!(target: "build", "{}", format!("Sitemap generated in {}", format_elapsed_time(sitemap_start.elapsed(), &FormatElapsedTimeOptions::default())).bold());
        } else {
            warn!(target: "build", "Sitemap generation is enabled but no base_url is set in BuildOptions. Either disable sitemap generation or set a base_url to enable it.");
        }
    }

    info!(target: "SKIP_FORMAT", "{}", "");
    info!(target: "build", "{}", format!("Build completed in {}", format_elapsed_time(build_start.elapsed(), &section_format_options)).bold());

    if let Some(clean_up_handle) = clean_up_handle {
        clean_up_handle.await?;
    }

    Ok(build_metadata)
}

fn add_sitemap_entry(
    sitemap_entries: &mut Vec<SitemapEntry>,
    base_url: Option<&str>,
    url: &str,
    route_path: &str,
    sitemap_metadata: &crate::sitemap::RouteSitemapMetadata,
    sitemap_options: &crate::sitemap::SitemapOptions,
) {
    // Skip if no base_url configured
    let Some(base_url) = base_url else {
        return;
    };

    // Skip if route is excluded or is a 404 page
    if sitemap_metadata.exclude.unwrap_or(false) || route_path.contains("404") {
        return;
    }

    // Construct full URL
    let full_url = if url == "/" {
        base_url.to_string()
    } else {
        format!("{}{}", base_url, url)
    };

    // Add entry
    sitemap_entries.push(SitemapEntry {
        loc: full_url,
        lastmod: None,
        changefreq: sitemap_metadata
            .changefreq
            .or(sitemap_options.default_changefreq),
        priority: sitemap_metadata
            .priority
            .or(sitemap_options.default_priority),
    });
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
