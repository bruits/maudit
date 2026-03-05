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
        self, HashAssetType, HashConfig, PrefetchPlugin, RouteAssets, Script, Style, StyleOptions,
        TailwindPlugin, calculate_hash, image_cache::ImageCache, prefetch,
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
use rolldown_plugin_replace::ReplacePlugin;
use rustc_hash::{FxHashMap, FxHashSet};

use crate::assets::Asset;
use crate::logging::{FormatElapsedTimeOptions, format_elapsed_time};
use rayon::prelude::*;

pub mod cache;
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

/// Record a rendered page's dependencies and assets into the build cache.
/// No-op when `new_cache` is None (incremental builds disabled).
fn record_page_cache_entry(
    new_cache: &mut Option<cache::BuildCache>,
    page_key: cache::PageKey,
    access_log: crate::content::tracked::ContentAccessLog,
    route_assets: &RouteAssets,
    output_file: PathBuf,
) {
    let Some(cache) = new_cache.as_mut() else {
        return;
    };
    cache.pages.insert(
        page_key,
        cache::PageCacheEntry {
            content_entries_read: access_log.entries_read,
            content_sources_iterated: access_log.sources_iterated,
            scripts: route_assets
                .scripts
                .iter()
                .map(|s| cache::CachedScript {
                    path: s.path.clone(),
                    hash: s.hash.clone(),
                    included: s.included,
                })
                .collect(),
            styles: route_assets
                .styles
                .iter()
                .map(|s| cache::CachedStyle {
                    path: s.path.clone(),
                    hash: s.hash.clone(),
                    included: s.included,
                    tailwind: s.tailwind,
                })
                .collect(),
            images: route_assets
                .images
                .iter()
                .map(|img| cache::CachedImage {
                    path: img.path.clone(),
                    hash: img.hash.clone(),
                })
                .collect(),
            output_file,
        },
    );
}

/// For a clean (non-dirty) page, reconstruct its Script and Style assets from cache
/// and add them to the global build sets. Images are skipped since they're already
/// in the output directory from the previous build.
fn restore_assets_from_cache(
    cached_entry: &cache::PageCacheEntry,
    route_assets_options: &assets::RouteAssetsOptions,
    build_scripts: &mut FxHashSet<Script>,
    build_styles: &mut FxHashSet<Style>,
) {
    for s in &cached_entry.scripts {
        build_scripts.insert(Script::new(
            s.path.clone(),
            s.included,
            s.hash.clone(),
            route_assets_options,
        ));
    }
    for s in &cached_entry.styles {
        build_styles.insert(Style::new(
            s.path.clone(),
            s.included,
            &StyleOptions {
                tailwind: s.tailwind,
            },
            s.hash.clone(),
            route_assets_options,
        ));
    }
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

    // When incremental is enabled, override clean_output_dir to false
    let should_clean = options.clean_output_dir && !options.incremental;

    let clean_up_handle = if should_clean {
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

    // --- Incremental build: compute state ---
    let incremental_state;
    let mut new_cache: Option<cache::BuildCache>;

    if options.incremental {
        let current_content_states: FxHashMap<String, cache::ContentSourceState> = content_sources
            .sources()
            .iter()
            .map(|s| {
                let entries = s.entry_file_info();
                (
                    s.get_name().to_string(),
                    cache::compute_content_source_state(&entries),
                )
            })
            .collect();

        incremental_state =
            cache::load_incremental_state(&options.cache_dir, &current_content_states);

        new_cache = Some(cache::BuildCache {
            version: cache::BUILD_CACHE_VERSION,
            binary_hash: cache::BuildCache::compute_binary_hash(),
            content_sources: current_content_states,
            ..Default::default()
        });
    } else {
        incremental_state = cache::IncrementalState::full_build();
        new_cache = None;
    };

    // --- Generate pages ---
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
    let mut rendered_count: usize = 0;
    let mut cached_count: usize = 0;
    let mut created_dirs: FxHashSet<PathBuf> = FxHashSet::default();

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

    // Serial page rendering loop.
    for route in routes {
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
                let params = PageParams::default();
                let (url, file_path) = cached_route.url_and_file_path(&params, &options.output_dir);

                // Try incremental cache hit (only when incremental is enabled)
                if new_cache.is_some() {
                    let page_key = cache::PageKey::new_static(base_path, None);

                    if !incremental_state.is_page_dirty(&page_key) {
                        if let Some(cached_entry) = incremental_state
                            .previous_cache
                            .as_ref()
                            .and_then(|c| c.pages.get(&page_key))
                        {
                            restore_assets_from_cache(
                                cached_entry,
                                &route_assets_options,
                                &mut build_pages_scripts,
                                &mut build_pages_styles,
                            );
                            if let Some(ref mut cache) = new_cache {
                                cache.pages.insert(page_key.clone(), cached_entry.clone());
                            }

                            info!(target: "pages", "{} -> {} (cached)", url, file_path.to_string_lossy().dimmed());

                            build_metadata.add_page(
                                base_path.clone(),
                                file_path.to_string_lossy().to_string(),
                                None,
                                true,
                            );

                            add_sitemap_entry(
                                &mut sitemap_entries,
                                normalized_base_url,
                                &url,
                                base_path,
                                &route.sitemap_metadata(),
                                &options.sitemap,
                            );

                            cached_count += 1;
                            // Skip to variants — use a block to scope the early exit
                        } else {
                            render_static_base_page(
                                route,
                                &cached_route,
                                base_path,
                                content_sources,
                                &route_assets_options,
                                &image_cache,
                                &default_scripts,
                                options,
                                &route_format_options,
                                normalized_base_url,
                                &url,
                                &file_path,
                                Some(page_key),
                                &mut build_pages_images,
                                &mut build_pages_scripts,
                                &mut build_pages_styles,
                                &mut sitemap_entries,
                                &mut build_metadata,
                                &mut new_cache,
                                &mut rendered_count,
                                &mut created_dirs,
                            )?;
                        }
                    } else {
                        render_static_base_page(
                            route,
                            &cached_route,
                            base_path,
                            content_sources,
                            &route_assets_options,
                            &image_cache,
                            &default_scripts,
                            options,
                            &route_format_options,
                            normalized_base_url,
                            &url,
                            &file_path,
                            Some(page_key),
                            &mut build_pages_images,
                            &mut build_pages_scripts,
                            &mut build_pages_styles,
                            &mut sitemap_entries,
                            &mut build_metadata,
                            &mut new_cache,
                            &mut rendered_count,
                            &mut created_dirs,
                        )?;
                    }
                } else {
                    // Non-incremental: render directly, no PageKey needed
                    render_static_base_page(
                        route,
                        &cached_route,
                        base_path,
                        content_sources,
                        &route_assets_options,
                        &image_cache,
                        &default_scripts,
                        options,
                        &route_format_options,
                        normalized_base_url,
                        &url,
                        &file_path,
                        None,
                        &mut build_pages_images,
                        &mut build_pages_scripts,
                        &mut build_pages_styles,
                        &mut sitemap_entries,
                        &mut build_metadata,
                        &mut new_cache,
                        &mut rendered_count,
                        &mut created_dirs,
                    )?;
                }
            } else {
                // Dynamic base route
                let mut pages_route_assets = RouteAssets::with_default_assets(
                    &route_assets_options,
                    Some(image_cache.clone()),
                    default_scripts.clone(),
                    vec![],
                );
                let pages = route.get_pages(&mut DynamicRouteContext::new(
                    content_sources,
                    &mut pages_route_assets,
                    None,
                ));

                if pages.is_empty() {
                    warn!(target: "build", "{} is a dynamic route, but its implementation of Route::pages returned an empty Vec. No pages will be generated for this route.", base_path.bold());
                    continue;
                } else {
                    // Log the pattern first
                    info!(target: "pages", "{}", base_path);

                    if new_cache.is_some() {
                        // Incremental: per-page RouteAssets for cache tracking
                        for page in pages {
                            let page_key = cache::PageKey::new(base_path, &page.0.0, None);
                            let (url, file_path) = cached_route.url_and_file_path(&page.0, &options.output_dir);

                            // Check if this page is clean
                            if !incremental_state.is_page_dirty(&page_key)
                                && let Some(cached_entry) = incremental_state
                                    .previous_cache
                                    .as_ref()
                                    .and_then(|c| c.pages.get(&page_key))
                            {
                                restore_assets_from_cache(
                                    cached_entry,
                                    &route_assets_options,
                                    &mut build_pages_scripts,
                                    &mut build_pages_styles,
                                );
                                if let Some(ref mut cache) = new_cache {
                                    cache.pages.insert(page_key.clone(), cached_entry.clone());
                                }

                                info!(target: "pages", "├─ {} (cached)", file_path.to_string_lossy().dimmed());

                                build_metadata.add_page(
                                    base_path.clone(),
                                    file_path.to_string_lossy().to_string(),
                                    Some(page.0.0.clone()),
                                    true,
                                );

                                add_sitemap_entry(
                                    &mut sitemap_entries,
                                    normalized_base_url,
                                    &url,
                                    base_path,
                                    &route.sitemap_metadata(),
                                    &options.sitemap,
                                );

                                cached_count += 1;
                                continue;
                            }

                            // Dirty or not in cache — render with per-page assets
                            let page_start = Instant::now();
                            let mut route_assets = RouteAssets::with_default_assets(
                                &route_assets_options,
                                Some(image_cache.clone()),
                                default_scripts.clone(),
                                vec![],
                            );

                            let mut page_ctx = PageContext::from_dynamic_route(
                                &page,
                                content_sources,
                                &mut route_assets,
                                &url,
                                &options.base_url,
                                None,
                            );
                            let content = route.build(&mut page_ctx)?;
                            let access_log = page_ctx.take_access_log();

                            write_route_file(&content, &file_path, &mut created_dirs)?;

                            info!(target: "pages", "├─ {} {}", file_path.to_string_lossy().dimmed(), format_elapsed_time(page_start.elapsed(), &route_format_options));

                            record_page_cache_entry(
                                &mut new_cache,
                                page_key,
                                access_log,
                                &route_assets,
                                file_path.clone(),
                            );

                            build_pages_images.extend(route_assets.images);
                            build_pages_scripts.extend(route_assets.scripts);
                            build_pages_styles.extend(route_assets.styles);

                            build_metadata.add_page(
                                base_path.clone(),
                                file_path.to_string_lossy().to_string(),
                                Some(page.0.0.clone()),
                                false,
                            );

                            add_sitemap_entry(
                                &mut sitemap_entries,
                                normalized_base_url,
                                &url,
                                base_path,
                                &route.sitemap_metadata(),
                                &options.sitemap,
                            );

                            rendered_count += 1;
                        }
                    } else {
                        // Non-incremental: shared RouteAssets, no PageKey overhead
                        let mut route_assets = RouteAssets::with_default_assets(
                            &route_assets_options,
                            Some(image_cache.clone()),
                            default_scripts.clone(),
                            vec![],
                        );

                        for page in pages {
                            let page_start = Instant::now();
                            let (url, file_path) = cached_route.url_and_file_path(&page.0, &options.output_dir);

                            let content = route.build(&mut PageContext::from_dynamic_route(
                                &page,
                                content_sources,
                                &mut route_assets,
                                &url,
                                &options.base_url,
                                None,
                            ))?;

                            write_route_file(&content, &file_path, &mut created_dirs)?;

                            info!(target: "pages", "├─ {} {}", file_path.to_string_lossy().dimmed(), format_elapsed_time(page_start.elapsed(), &route_format_options));

                            build_metadata.add_page(
                                base_path.clone(),
                                file_path.to_string_lossy().to_string(),
                                Some(page.0.0.clone()),
                                false,
                            );

                            add_sitemap_entry(
                                &mut sitemap_entries,
                                normalized_base_url,
                                &url,
                                base_path,
                                &route.sitemap_metadata(),
                                &options.sitemap,
                            );

                            rendered_count += 1;
                        }

                        build_pages_images.extend(route_assets.images);
                        build_pages_scripts.extend(route_assets.scripts);
                        build_pages_styles.extend(route_assets.styles);
                    }
                }
            }
        }

        // Handle variants
        for (variant_id, variant_path) in variants {
            let variant_params = extract_params_from_raw_route(&variant_path);

            if variant_params.is_empty() {
                // Static variant
                let params = PageParams::default();
                let (url, file_path) = cached_route.variant_url_and_file_path(&params, &options.output_dir, &variant_id)?;

                // Try incremental cache hit
                if new_cache.is_some() {
                    let page_key = cache::PageKey::new_static(&variant_path, Some(&variant_id));

                    if !incremental_state.is_page_dirty(&page_key)
                        && let Some(cached_entry) = incremental_state
                            .previous_cache
                            .as_ref()
                            .and_then(|c| c.pages.get(&page_key))
                    {
                        restore_assets_from_cache(
                            cached_entry,
                            &route_assets_options,
                            &mut build_pages_scripts,
                            &mut build_pages_styles,
                        );
                        if let Some(ref mut cache) = new_cache {
                            cache.pages.insert(page_key.clone(), cached_entry.clone());
                        }

                        info!(target: "pages", "├─ {} (cached)", file_path.to_string_lossy().dimmed());

                        build_metadata.add_page(
                            variant_path.clone(),
                            file_path.to_string_lossy().to_string(),
                            None,
                            true,
                        );

                        add_sitemap_entry(
                            &mut sitemap_entries,
                            normalized_base_url,
                            &url,
                            &variant_path,
                            &route.sitemap_metadata(),
                            &options.sitemap,
                        );

                        cached_count += 1;
                        continue;
                    }

                    // Dirty or not in cache — render
                    let variant_start = Instant::now();
                    let mut route_assets = RouteAssets::with_default_assets(
                        &route_assets_options,
                        Some(image_cache.clone()),
                        default_scripts.clone(),
                        vec![],
                    );

                    let mut page_ctx = PageContext::from_static_route(
                        content_sources,
                        &mut route_assets,
                        &url,
                        &options.base_url,
                        Some(variant_id.clone()),
                    );
                    let result = route.build(&mut page_ctx)?;
                    let access_log = page_ctx.take_access_log();

                    write_route_file(&result, &file_path, &mut created_dirs)?;

                    info!(target: "pages", "├─ {} {}", file_path.to_string_lossy().dimmed(), format_elapsed_time(variant_start.elapsed(), &route_format_options));

                    record_page_cache_entry(
                        &mut new_cache,
                        page_key,
                        access_log,
                        &route_assets,
                        file_path.clone(),
                    );

                    build_pages_images.extend(route_assets.images);
                    build_pages_scripts.extend(route_assets.scripts);
                    build_pages_styles.extend(route_assets.styles);
                } else {
                    // Non-incremental: render directly
                    let variant_start = Instant::now();
                    let mut route_assets = RouteAssets::with_default_assets(
                        &route_assets_options,
                        Some(image_cache.clone()),
                        default_scripts.clone(),
                        vec![],
                    );

                    let mut page_ctx = PageContext::from_static_route(
                        content_sources,
                        &mut route_assets,
                        &url,
                        &options.base_url,
                        Some(variant_id.clone()),
                    );
                    let result = route.build(&mut page_ctx)?;

                    write_route_file(&result, &file_path, &mut created_dirs)?;

                    info!(target: "pages", "├─ {} {}", file_path.to_string_lossy().dimmed(), format_elapsed_time(variant_start.elapsed(), &route_format_options));

                    build_pages_images.extend(route_assets.images);
                    build_pages_scripts.extend(route_assets.scripts);
                    build_pages_styles.extend(route_assets.styles);
                }

                build_metadata.add_page(
                    variant_path.clone(),
                    file_path.to_string_lossy().to_string(),
                    None,
                    false,
                );

                add_sitemap_entry(
                    &mut sitemap_entries,
                    normalized_base_url,
                    &url,
                    &variant_path,
                    &route.sitemap_metadata(),
                    &options.sitemap,
                );

                rendered_count += 1;
            } else {
                // Dynamic variant
                let mut pages_route_assets = RouteAssets::with_default_assets(
                    &route_assets_options,
                    Some(image_cache.clone()),
                    default_scripts.clone(),
                    vec![],
                );
                let pages = route.get_pages(&mut DynamicRouteContext::new(
                    content_sources,
                    &mut pages_route_assets,
                    Some(&variant_id),
                ));

                if pages.is_empty() {
                    warn!(target: "build", "Variant {} has dynamic parameters but Route::pages returned an empty Vec.", variant_id.bold());
                } else {
                    // Log the variant pattern first
                    info!(target: "pages", "├─ {}", variant_path);

                    if new_cache.is_some() {
                        // Incremental: per-page RouteAssets for cache tracking
                        for page in pages {
                            let page_key =
                                cache::PageKey::new(&variant_path, &page.0.0, Some(&variant_id));
                            let (url, file_path) = cached_route.variant_url_and_file_path(&page.0, &options.output_dir, &variant_id)?;

                            if !incremental_state.is_page_dirty(&page_key)
                                && let Some(cached_entry) = incremental_state
                                    .previous_cache
                                    .as_ref()
                                    .and_then(|c| c.pages.get(&page_key))
                            {
                                restore_assets_from_cache(
                                    cached_entry,
                                    &route_assets_options,
                                    &mut build_pages_scripts,
                                    &mut build_pages_styles,
                                );
                                if let Some(ref mut cache) = new_cache {
                                    cache.pages.insert(page_key.clone(), cached_entry.clone());
                                }

                                info!(target: "pages", "│  ├─ {} (cached)", file_path.to_string_lossy().dimmed());

                                build_metadata.add_page(
                                    variant_path.clone(),
                                    file_path.to_string_lossy().to_string(),
                                    Some(page.0.0.clone()),
                                    true,
                                );

                                add_sitemap_entry(
                                    &mut sitemap_entries,
                                    normalized_base_url,
                                    &url,
                                    &variant_path,
                                    &route.sitemap_metadata(),
                                    &options.sitemap,
                                );

                                cached_count += 1;
                                continue;
                            }

                            let variant_page_start = Instant::now();
                            let mut route_assets = RouteAssets::with_default_assets(
                                &route_assets_options,
                                Some(image_cache.clone()),
                                default_scripts.clone(),
                                vec![],
                            );

                            let mut page_ctx = PageContext::from_dynamic_route(
                                &page,
                                content_sources,
                                &mut route_assets,
                                &url,
                                &options.base_url,
                                Some(variant_id.clone()),
                            );
                            let content = route.build(&mut page_ctx)?;
                            let access_log = page_ctx.take_access_log();

                            write_route_file(&content, &file_path, &mut created_dirs)?;

                            info!(target: "pages", "│  ├─ {} {}", file_path.to_string_lossy().dimmed(), format_elapsed_time(variant_page_start.elapsed(), &route_format_options));

                            record_page_cache_entry(
                                &mut new_cache,
                                page_key,
                                access_log,
                                &route_assets,
                                file_path.clone(),
                            );

                            build_pages_images.extend(route_assets.images);
                            build_pages_scripts.extend(route_assets.scripts);
                            build_pages_styles.extend(route_assets.styles);

                            build_metadata.add_page(
                                variant_path.clone(),
                                file_path.to_string_lossy().to_string(),
                                Some(page.0.0.clone()),
                                false,
                            );

                            add_sitemap_entry(
                                &mut sitemap_entries,
                                normalized_base_url,
                                &url,
                                &variant_path,
                                &route.sitemap_metadata(),
                                &options.sitemap,
                            );

                            rendered_count += 1;
                        }
                    } else {
                        // Non-incremental: shared RouteAssets
                        let mut route_assets = RouteAssets::with_default_assets(
                            &route_assets_options,
                            Some(image_cache.clone()),
                            default_scripts.clone(),
                            vec![],
                        );

                        for page in pages {
                            let variant_page_start = Instant::now();
                            let (url, file_path) = cached_route.variant_url_and_file_path(&page.0, &options.output_dir, &variant_id)?;

                            let content = route.build(&mut PageContext::from_dynamic_route(
                                &page,
                                content_sources,
                                &mut route_assets,
                                &url,
                                &options.base_url,
                                Some(variant_id.clone()),
                            ))?;

                            write_route_file(&content, &file_path, &mut created_dirs)?;

                            info!(target: "pages", "│  ├─ {} {}", file_path.to_string_lossy().dimmed(), format_elapsed_time(variant_page_start.elapsed(), &route_format_options));

                            build_metadata.add_page(
                                variant_path.clone(),
                                file_path.to_string_lossy().to_string(),
                                Some(page.0.0.clone()),
                                false,
                            );

                            add_sitemap_entry(
                                &mut sitemap_entries,
                                normalized_base_url,
                                &url,
                                &variant_path,
                                &route.sitemap_metadata(),
                                &options.sitemap,
                            );

                            rendered_count += 1;
                        }

                        build_pages_images.extend(route_assets.images);
                        build_pages_scripts.extend(route_assets.scripts);
                        build_pages_styles.extend(route_assets.styles);
                    }
                }
            }
        }
    }

    let page_count = rendered_count + cached_count;
    if cached_count > 0 {
        info!(target: "pages", "{}", format!("generated {} pages ({} rendered, {} cached) in {}", page_count, rendered_count, cached_count, format_elapsed_time(pages_start.elapsed(), &section_format_options)).bold());
    } else {
        info!(target: "pages", "{}", format!("generated {} pages in {}", page_count, format_elapsed_time(pages_start.elapsed(), &section_format_options)).bold());
    }

    // Populate asset_file_hashes in the new cache from all pages (cached + rendered)
    if let Some(ref mut cache) = new_cache {
        for page_entry in cache.pages.values() {
            for img in &page_entry.images {
                cache
                    .asset_file_hashes
                    .entry(img.path.clone())
                    .or_insert_with(|| cache::hash_file_content(&img.path).unwrap_or_default());
            }
            for script in &page_entry.scripts {
                cache
                    .asset_file_hashes
                    .entry(script.path.clone())
                    .or_insert_with(|| cache::hash_file_content(&script.path).unwrap_or_default());
            }
            for style in &page_entry.styles {
                cache
                    .asset_file_hashes
                    .entry(style.path.clone())
                    .or_insert_with(|| cache::hash_file_content(&style.path).unwrap_or_default());
            }
        }
    }

    if (!build_pages_images.is_empty())
        || !build_pages_styles.is_empty()
        || !build_pages_scripts.is_empty()
    {
        fs::create_dir_all(&route_assets_options.output_assets_dir)?;
    }

    // Determine if rebundling is needed (incremental optimization)
    let should_bundle = if !incremental_state.is_full_build() {
        let current_bundled_scripts: FxHashSet<cache::SerializedAssetRef> = build_pages_scripts
            .iter()
            .map(|s| cache::SerializedAssetRef {
                path: s.path.clone(),
                hash: s.hash.clone(),
            })
            .collect();
        let current_bundled_styles: FxHashSet<cache::SerializedAssetRef> = build_pages_styles
            .iter()
            .map(|s| cache::SerializedAssetRef {
                path: s.path.clone(),
                hash: s.hash.clone(),
            })
            .collect();

        let has_tailwind = build_pages_styles.iter().any(|s| s.tailwind);
        let content_changed = rendered_count > 0;

        let needs_bundle = if has_tailwind && content_changed {
            // Tailwind output depends on classes used in source files, so any content
            // change could introduce new classes — must rebundle
            true
        } else if let Some(prev) = &incremental_state.previous_cache {
            cache::needs_rebundle(
                &prev.bundled_scripts,
                &prev.bundled_styles,
                &current_bundled_scripts,
                &current_bundled_styles,
            )
        } else {
            true
        };

        // Save current bundle state in new cache
        if let Some(ref mut cache) = new_cache {
            cache.bundled_scripts = current_bundled_scripts.into_iter().collect();
            cache.bundled_styles = current_bundled_styles.into_iter().collect();
        }

        needs_bundle
    } else {
        // Full build: save bundle state and always bundle
        if let Some(ref mut cache) = new_cache {
            cache.bundled_scripts = build_pages_scripts
                .iter()
                .map(|s| cache::SerializedAssetRef {
                    path: s.path.clone(),
                    hash: s.hash.clone(),
                })
                .collect();
            cache.bundled_styles = build_pages_styles
                .iter()
                .map(|s| cache::SerializedAssetRef {
                    path: s.path.clone(),
                    hash: s.hash.clone(),
                })
                .collect();
        }
        true
    };

    if should_bundle && (!build_pages_styles.is_empty() || !build_pages_scripts.is_empty()) {
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
                    Arc::new(ReplacePlugin::new(FxHashMap::default())?),
                ],
            )?;

            let _result = bundler.write().await?;

            // TODO: Add outputted chunks to build_metadata
        }

        info!(target: "build", "{}", format!("Assets generated in {}", format_elapsed_time(assets_start.elapsed(), &section_format_options)).bold());
    } else if !should_bundle && (!build_pages_styles.is_empty() || !build_pages_scripts.is_empty())
    {
        info!(target: "build", "Assets unchanged, skipping bundling");
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

    // Delete stale output files (pages that existed in previous cache but no longer generated)
    if !incremental_state.is_full_build()
        && let Some(ref cache) = new_cache
    {
        let current_page_keys: FxHashSet<cache::PageKey> = cache.pages.keys().cloned().collect();
        if let Some(prev_cache) = &incremental_state.previous_cache {
            let stale = cache::find_stale_pages(&prev_cache.pages, &current_page_keys);
            for stale_key in &stale {
                if let Some(entry) = prev_cache.pages.get(stale_key)
                    && fs::remove_file(&entry.output_file).is_ok()
                {
                    info!(target: "build", "Removed stale output: {}", entry.output_file.display());
                }
            }
        }
    }

    info!(target: "SKIP_FORMAT", "{}", "");
    info!(target: "build", "{}", format!("Build completed in {}", format_elapsed_time(build_start.elapsed(), &section_format_options)).bold());

    // Save build cache (after timing, since this is a post-build bookkeeping step)
    if let Some(cache) = new_cache {
        if let Err(e) = cache.save(&options.cache_dir) {
            warn!(target: "build", "Failed to save build cache: {}", e);
        } else {
            debug!(target: "build", "Build cache saved to {}", options.cache_dir.display());
        }
    }

    if let Some(clean_up_handle) = clean_up_handle {
        clean_up_handle.await?;
    }

    Ok(build_metadata)
}

/// Render a static base page (extracted to reduce duplication between dirty/not-in-cache paths).
#[allow(clippy::too_many_arguments, clippy::mutable_key_type)]
fn render_static_base_page(
    route: &&dyn FullRoute,
    _cached_route: &CachedRoute,
    base_path: &str,
    content_sources: &ContentSources,
    route_assets_options: &assets::RouteAssetsOptions,
    image_cache: &ImageCache,
    default_scripts: &[Script],
    options: &BuildOptions,
    route_format_options: &FormatElapsedTimeOptions,
    normalized_base_url: Option<&str>,
    url: &str,
    file_path: &PathBuf,
    page_key: Option<cache::PageKey>,
    build_pages_images: &mut FxHashSet<assets::Image>,
    build_pages_scripts: &mut FxHashSet<assets::Script>,
    build_pages_styles: &mut FxHashSet<assets::Style>,
    sitemap_entries: &mut Vec<SitemapEntry>,
    build_metadata: &mut BuildOutput,
    new_cache: &mut Option<cache::BuildCache>,
    rendered_count: &mut usize,
    created_dirs: &mut FxHashSet<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    let page_start = Instant::now();
    let mut route_assets = RouteAssets::with_default_assets(
        route_assets_options,
        Some(image_cache.clone()),
        default_scripts.to_vec(),
        vec![],
    );

    let url_string = url.to_string();
    let mut page_ctx = PageContext::from_static_route(
        content_sources,
        &mut route_assets,
        &url_string,
        &options.base_url,
        None,
    );
    let result = route.build(&mut page_ctx)?;
    let access_log = page_ctx.take_access_log();

    write_route_file(&result, file_path, created_dirs)?;

    info!(target: "pages", "{} -> {} {}", url, file_path.to_string_lossy().dimmed(), format_elapsed_time(page_start.elapsed(), route_format_options));

    if let Some(page_key) = page_key {
        record_page_cache_entry(
            new_cache,
            page_key,
            access_log,
            &route_assets,
            file_path.clone(),
        );
    }

    build_pages_images.extend(route_assets.images);
    build_pages_scripts.extend(route_assets.scripts);
    build_pages_styles.extend(route_assets.styles);

    build_metadata.add_page(
        base_path.to_string(),
        file_path.to_string_lossy().to_string(),
        None,
        false,
    );

    add_sitemap_entry(
        sitemap_entries,
        normalized_base_url,
        url,
        base_path,
        &route.sitemap_metadata(),
        &options.sitemap,
    );

    *rendered_count += 1;
    Ok(())
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

fn write_route_file(
    content: &[u8],
    file_path: &PathBuf,
    created_dirs: &mut FxHashSet<PathBuf>,
) -> Result<(), io::Error> {
    if let Some(parent_dir) = file_path.parent()
        && created_dirs.insert(parent_dir.to_path_buf())
    {
        fs::create_dir_all(parent_dir)?;
    }

    fs::write(file_path, content)?;

    Ok(())
}
