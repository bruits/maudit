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
    build::{
        images::process_image,
        options::PrefetchStrategy,
        state::{BuildState, RouteIdentifier},
    },
    content::{ContentSources, finish_tracking_content_files, start_tracking_content_files},
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
use rolldown_common::Output;
use rolldown_plugin_replace::ReplacePlugin;
use rustc_hash::{FxHashMap, FxHashSet};

use crate::assets::Asset;
use crate::logging::{FormatElapsedTimeOptions, format_elapsed_time};
use rayon::prelude::*;

pub mod images;
pub mod metadata;
pub mod options;
pub mod state;

/// Helper to check if a route should be rebuilt during incremental builds.
/// Returns `true` for full builds (when `routes_to_rebuild` is `None`).
fn should_rebuild_route(
    route_id: Option<&RouteIdentifier>,
    routes_to_rebuild: &Option<FxHashSet<RouteIdentifier>>,
) -> bool {
    match routes_to_rebuild {
        Some(set) => {
            // Incremental build - need route_id to check
            let route_id = route_id.expect("route_id required for incremental builds");
            let result = set.contains(route_id);
            if !result {
                trace!(target: "build", "Skipping route {:?} (not in rebuild set)", route_id);
            }
            result
        }
        None => true, // Full build - always rebuild
    }
}

/// Helper to track all assets and source files used by a route.
/// Only performs work when incremental builds are enabled and route_id is provided.
fn track_route_assets(
    build_state: &mut BuildState,
    route_id: Option<&RouteIdentifier>,
    route_assets: &RouteAssets,
) {
    // Skip tracking entirely when route_id is not provided (incremental disabled)
    let Some(route_id) = route_id else {
        return;
    };

    // Track images
    for image in &route_assets.images {
        if let Ok(canonical) = image.path().canonicalize() {
            build_state.track_asset(canonical, route_id.clone());
        }
    }

    // Track scripts
    for script in &route_assets.scripts {
        if let Ok(canonical) = script.path().canonicalize() {
            build_state.track_asset(canonical, route_id.clone());
        }
    }

    // Track styles
    for style in &route_assets.styles {
        if let Ok(canonical) = style.path().canonicalize() {
            build_state.track_asset(canonical, route_id.clone());
        }
    }
}

/// Helper to track the source file where a route is defined.
/// Only performs work when incremental builds are enabled and route_id is provided.
fn track_route_source_file(
    build_state: &mut BuildState,
    route_id: Option<&RouteIdentifier>,
    source_file: &str,
) {
    // Skip tracking entirely when route_id is not provided (incremental disabled)
    let Some(route_id) = route_id else {
        return;
    };

    // The file!() macro returns a path relative to the cargo workspace root.
    // We need to canonicalize it to match against changed file paths (which are absolute).
    let source_path = PathBuf::from(source_file);

    // Try direct canonicalization first (works if CWD is workspace root)
    if let Ok(canonical) = source_path.canonicalize() {
        build_state.track_source_file(canonical, route_id.clone());
        return;
    }

    // The file!() macro path is relative to the workspace root at compile time.
    // At runtime, we're typically running from the package directory.
    // Try to find the file by walking up from CWD until we find it.
    if let Ok(cwd) = std::env::current_dir() {
        let mut current = cwd.as_path();
        loop {
            let candidate = current.join(&source_path);
            if let Ok(canonical) = candidate.canonicalize() {
                build_state.track_source_file(canonical, route_id.clone());
                return;
            }
            match current.parent() {
                Some(parent) => current = parent,
                None => break,
            }
        }
    }

    // Last resort: store the relative path (won't match absolute changed files)
    debug!(target: "build", "Could not canonicalize source file path: {}", source_file);
    build_state.track_source_file(source_path, route_id.clone());
}

/// Helper to track content files accessed during page rendering.
/// Only performs work when incremental builds are enabled and route_id is provided.
/// This should be called after `finish_tracking_content_files()` to get the accessed files.
fn track_route_content_files(
    build_state: &mut BuildState,
    route_id: Option<&RouteIdentifier>,
    accessed_files: Option<FxHashSet<PathBuf>>,
) {
    // Skip tracking entirely when route_id is not provided (incremental disabled)
    let Some(route_id) = route_id else {
        return;
    };

    // Skip if no files were tracked
    let Some(files) = accessed_files else {
        return;
    };

    for file_path in files {
        build_state.track_content_file(file_path, route_id.clone());
    }
}

pub fn execute_build(
    routes: &[&dyn FullRoute],
    content_sources: &mut ContentSources,
    options: &BuildOptions,
    changed_files: Option<&[PathBuf]>,
    async_runtime: &tokio::runtime::Runtime,
) -> Result<BuildOutput, Box<dyn std::error::Error>> {
    async_runtime.block_on(async { build(routes, content_sources, options, changed_files).await })
}

pub async fn build(
    routes: &[&dyn FullRoute],
    content_sources: &mut ContentSources,
    options: &BuildOptions,
    changed_files: Option<&[PathBuf]>,
) -> Result<BuildOutput, Box<dyn std::error::Error>> {
    let build_start = Instant::now();
    let mut build_metadata = BuildOutput::new(build_start);

    // Create a directory for the output
    trace!(target: "build", "Setting up required directories...");

    // Use cache directory from options
    let build_cache_dir = &options.cache_dir;

    // Load build state for incremental builds (only if incremental is enabled)
    let mut build_state = if options.incremental {
        BuildState::load(build_cache_dir).unwrap_or_else(|e| {
            debug!(target: "build", "Failed to load build state: {}", e);
            BuildState::new()
        })
    } else {
        BuildState::new()
    };

    debug!(target: "build", "Loaded build state with {} asset mappings, {} source mappings, {} content file mappings", build_state.asset_to_routes.len(), build_state.source_to_routes.len(), build_state.content_file_to_routes.len());
    debug!(target: "build", "options.incremental: {}, changed_files.is_some(): {}", options.incremental, changed_files.is_some());

    // Determine if this is an incremental build
    // We need either asset mappings OR source file mappings to do incremental builds
    let has_build_state =
        !build_state.asset_to_routes.is_empty() || !build_state.source_to_routes.is_empty();
    let is_incremental = options.incremental && changed_files.is_some() && has_build_state;

    let routes_to_rebuild = if is_incremental {
        let changed = changed_files.unwrap();
        info!(target: "build", "Incremental build: {} files changed", changed.len());
        info!(target: "build", "Changed files: {:?}", changed);

        info!(target: "build", "Build state has {} asset mappings, {} source mappings, {} content file mappings", build_state.asset_to_routes.len(), build_state.source_to_routes.len(), build_state.content_file_to_routes.len());

        match build_state.get_affected_routes(changed) {
            Some(affected) => {
                info!(target: "build", "Rebuilding {} affected routes", affected.len());
                info!(target: "build", "Affected routes: {:?}", affected);
                Some(affected)
            }
            None => {
                // Some changed files weren't tracked (e.g., include_str! dependencies)
                // Fall back to full rebuild to ensure correctness
                info!(target: "build", "Untracked files changed, falling back to full rebuild");
                build_state.clear();
                None
            }
        }
    } else {
        if changed_files.is_some() {
            info!(target: "build", "Full build (first run after recompilation)");
        }
        // Full build - clear old state
        build_state.clear();
        None
    };

    // Check if we should rebundle during incremental builds
    // Rebundle if a changed file is either:
    // 1. A direct bundler input (entry point)
    // 2. A transitive dependency tracked in asset_to_routes (any file the bundler processed)
    let should_rebundle = if is_incremental && !build_state.bundler_inputs.is_empty() {
        let changed = changed_files.unwrap();
        let should = changed.iter().any(|changed_file| {
            // Check if it's a direct bundler input
            let is_bundler_input = build_state.bundler_inputs.iter().any(|bundler_input| {
                if let (Ok(changed_canonical), Ok(bundler_canonical)) = (
                    changed_file.canonicalize(),
                    PathBuf::from(bundler_input).canonicalize(),
                ) {
                    changed_canonical == bundler_canonical
                } else {
                    false
                }
            });

            if is_bundler_input {
                return true;
            }

            // Check if it's a transitive dependency tracked by the bundler
            // (JS/TS modules, CSS files, or assets like images/fonts referenced via url())
            if let Ok(canonical) = changed_file.canonicalize() {
                return build_state.asset_to_routes.contains_key(&canonical);
            }

            false
        });

        if should {
            info!(target: "build", "Rebundling needed: changed file affects bundled assets");
        } else {
            info!(target: "build", "Skipping bundler: no changed files affect bundled assets");
        }

        should
    } else {
        // Not incremental or no previous bundler inputs
        false
    };

    let clean_up_handle = if options.clean_output_dir && !is_incremental {
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
    let image_cache = ImageCache::with_cache_dir(options.assets_cache_dir());
    let _ = fs::create_dir_all(image_cache.get_cache_dir());

    // Create route_assets_options with the image cache
    let route_assets_options = options.route_assets_options();

    info!(target: "build", "Output directory: {}", options.output_dir.display());

    let content_sources_start = Instant::now();
    print_title("initializing content sources");

    // Determine which content sources need to be initialized
    // For incremental builds, only re-init sources whose files have changed
    let sources_to_init: Option<FxHashSet<String>> = if is_incremental {
        if let Some(changed) = changed_files {
            build_state.get_affected_content_sources(changed)
        } else {
            None // Full init
        }
    } else {
        None // Full init
    };

    // Initialize content sources (all or selective)
    let initialized_sources: Vec<String> = match &sources_to_init {
        Some(source_names) if !source_names.is_empty() => {
            info!(target: "content", "Selectively initializing {} content source(s): {:?}", source_names.len(), source_names);
            
            // Clear mappings for sources being re-initialized before init
            build_state.clear_content_mappings_for_sources(source_names);
            
            // Initialize only the affected sources
            let mut initialized = Vec::new();
            for source in content_sources.sources_mut() {
                if source_names.contains(source.get_name()) {
                    let source_start = Instant::now();
                    source.init();
                    info!(target: "content", "{} initialized in {}", source.get_name(), format_elapsed_time(source_start.elapsed(), &FormatElapsedTimeOptions::default()));
                    initialized.push(source.get_name().to_string());
                } else {
                    info!(target: "content", "{} (unchanged, skipped)", source.get_name());
                }
            }
            initialized
        }
        Some(_) => {
            // Empty set means no content files changed, skip all initialization
            info!(target: "content", "No content files changed, skipping content source initialization");
            Vec::new()
        }
        None => {
            // Full initialization (first build, unknown files, or non-incremental)
            info!(target: "content", "Initializing all content sources");
            
            // Clear all content mappings for full init
            build_state.clear_content_file_mappings();
            build_state.content_file_to_source.clear();
            
            let mut initialized = Vec::new();
            for source in content_sources.sources_mut() {
                let source_start = Instant::now();
                source.init();
                info!(target: "content", "{} initialized in {}", source.get_name(), format_elapsed_time(source_start.elapsed(), &FormatElapsedTimeOptions::default()));
                initialized.push(source.get_name().to_string());
            }
            initialized
        }
    };

    // Track file->source mappings for all initialized sources
    for source in content_sources.sources() {
        if initialized_sources.contains(&source.get_name().to_string()) {
            let source_name = source.get_name().to_string();
            for file_path in source.get_entry_file_paths() {
                build_state.track_content_file_source(file_path, source_name.clone());
            }
        }
    }

    info!(target: "content", "{}", format!("Content sources initialized in {}", format_elapsed_time(
        content_sources_start.elapsed(),
        &FormatElapsedTimeOptions::default(),
    )).bold());

    // Clear content file->routes mappings for routes being rebuilt
    // (so they get fresh tracking during this build)
    if let Some(ref routes) = routes_to_rebuild {
        build_state.clear_content_file_mappings_for_routes(routes);
    }

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
                // Only create RouteIdentifier when incremental builds are enabled
                let route_id = if options.incremental {
                    Some(RouteIdentifier::base(base_path.clone(), None))
                } else {
                    None
                };

                // Check if we need to rebuild this route
                if should_rebuild_route(route_id.as_ref(), &routes_to_rebuild) {
                    let mut route_assets = RouteAssets::with_default_assets(
                        &route_assets_options,
                        Some(image_cache.clone()),
                        default_scripts.clone(),
                        vec![],
                    );

                    let params = PageParams::default();
                    let url = cached_route.url(&params);

                    // Start tracking content file access for incremental builds
                    if options.incremental {
                        start_tracking_content_files();
                    }

                    let result = route.build(&mut PageContext::from_static_route(
                        content_sources,
                        &mut route_assets,
                        &url,
                        &options.base_url,
                        None,
                    ))?;

                    // Finish tracking and record accessed content files
                    let accessed_files = if options.incremental {
                        finish_tracking_content_files()
                    } else {
                        None
                    };

                    let file_path = cached_route.file_path(&params, &options.output_dir);

                    write_route_file(&result, &file_path)?;

                    info!(target: "pages", "{} -> {} {}", url, file_path.to_string_lossy().dimmed(), format_elapsed_time(route_start.elapsed(), &route_format_options));

                    // Track assets, source file, and content files for this route
                    track_route_assets(&mut build_state, route_id.as_ref(), &route_assets);
                    track_route_source_file(&mut build_state, route_id.as_ref(), route.source_file());
                    track_route_content_files(&mut build_state, route_id.as_ref(), accessed_files);

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
                    trace!(target: "build", "Skipping unchanged route: {}", base_path);
                }
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
                        // Only create RouteIdentifier when incremental builds are enabled
                        let route_id = if options.incremental {
                            Some(RouteIdentifier::base(base_path.clone(), Some(page.0.0.clone())))
                        } else {
                            None
                        };

                        // Check if we need to rebuild this specific page
                        if should_rebuild_route(route_id.as_ref(), &routes_to_rebuild) {
                            let page_start = Instant::now();
                            let url = cached_route.url(&page.0);
                            let file_path = cached_route.file_path(&page.0, &options.output_dir);

                            // Start tracking content file access for incremental builds
                            if options.incremental {
                                start_tracking_content_files();
                            }

                            let content = route.build(&mut PageContext::from_dynamic_route(
                                &page,
                                content_sources,
                                &mut route_assets,
                                &url,
                                &options.base_url,
                                None,
                            ))?;

                            // Finish tracking and record accessed content files
                            let accessed_files = if options.incremental {
                                finish_tracking_content_files()
                            } else {
                                None
                            };

                            write_route_file(&content, &file_path)?;

                            info!(target: "pages", "├─ {} {}", file_path.to_string_lossy().dimmed(), format_elapsed_time(page_start.elapsed(), &route_format_options));

                            // Track assets, source file, and content files for this page
                            track_route_assets(&mut build_state, route_id.as_ref(), &route_assets);
                            track_route_source_file(&mut build_state, route_id.as_ref(), route.source_file());
                            track_route_content_files(&mut build_state, route_id.as_ref(), accessed_files);

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
                        } else {
                            trace!(target: "build", "Skipping unchanged page: {} with params {:?}", base_path, page.0.0);
                        }
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
                // Only create RouteIdentifier when incremental builds are enabled
                let route_id = if options.incremental {
                    Some(RouteIdentifier::variant(variant_id.clone(), variant_path.clone(), None))
                } else {
                    None
                };

                // Check if we need to rebuild this variant
                if should_rebuild_route(route_id.as_ref(), &routes_to_rebuild) {
                    let mut route_assets = RouteAssets::with_default_assets(
                        &route_assets_options,
                        Some(image_cache.clone()),
                        default_scripts.clone(),
                        vec![],
                    );

                    let params = PageParams::default();
                    let url = cached_route.variant_url(&params, &variant_id)?;
                    let file_path = cached_route.variant_file_path(
                        &params,
                        &options.output_dir,
                        &variant_id,
                    )?;

                    // Start tracking content file access for incremental builds
                    if options.incremental {
                        start_tracking_content_files();
                    }

                    let result = route.build(&mut PageContext::from_static_route(
                        content_sources,
                        &mut route_assets,
                        &url,
                        &options.base_url,
                        Some(variant_id.clone()),
                    ))?;

                    // Finish tracking and record accessed content files
                    let accessed_files = if options.incremental {
                        finish_tracking_content_files()
                    } else {
                        None
                    };

                    write_route_file(&result, &file_path)?;

                    info!(target: "pages", "├─ {} {}", file_path.to_string_lossy().dimmed(), format_elapsed_time(variant_start.elapsed(), &route_format_options));

                    // Track assets, source file, and content files for this variant
                    track_route_assets(&mut build_state, route_id.as_ref(), &route_assets);
                    track_route_source_file(&mut build_state, route_id.as_ref(), route.source_file());
                    track_route_content_files(&mut build_state, route_id.as_ref(), accessed_files);

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
                    trace!(target: "build", "Skipping unchanged variant: {}", variant_path);
                }
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
                        // Only create RouteIdentifier when incremental builds are enabled
                        let route_id = if options.incremental {
                            Some(RouteIdentifier::variant(
                                variant_id.clone(),
                                variant_path.clone(),
                                Some(page.0.0.clone()),
                            ))
                        } else {
                            None
                        };

                        // Check if we need to rebuild this specific variant page
                        if should_rebuild_route(route_id.as_ref(), &routes_to_rebuild) {
                            let variant_page_start = Instant::now();
                            let url = cached_route.variant_url(&page.0, &variant_id)?;
                            let file_path = cached_route.variant_file_path(
                                &page.0,
                                &options.output_dir,
                                &variant_id,
                            )?;

                            // Start tracking content file access for incremental builds
                            if options.incremental {
                                start_tracking_content_files();
                            }

                            let content = route.build(&mut PageContext::from_dynamic_route(
                                &page,
                                content_sources,
                                &mut route_assets,
                                &url,
                                &options.base_url,
                                Some(variant_id.clone()),
                            ))?;

                            // Finish tracking and record accessed content files
                            let accessed_files = if options.incremental {
                                finish_tracking_content_files()
                            } else {
                                None
                            };

                            write_route_file(&content, &file_path)?;

                            info!(target: "pages", "│  ├─ {} {}", file_path.to_string_lossy().dimmed(), format_elapsed_time(variant_page_start.elapsed(), &route_format_options));

                            // Track assets, source file, and content files for this variant page
                            track_route_assets(&mut build_state, route_id.as_ref(), &route_assets);
                            track_route_source_file(&mut build_state, route_id.as_ref(), route.source_file());
                            track_route_content_files(&mut build_state, route_id.as_ref(), accessed_files);

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
                        } else {
                            trace!(target: "build", "Skipping unchanged variant page: {} with params {:?}", variant_path, page.0.0);
                        }
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

    if !build_pages_styles.is_empty()
        || !build_pages_scripts.is_empty()
        || (is_incremental && should_rebundle)
    {
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

        let mut bundler_inputs = build_pages_scripts
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

        // During incremental builds, merge with previous bundler inputs
        // to ensure we bundle all assets, not just from rebuilt routes
        if is_incremental && !build_state.bundler_inputs.is_empty() {
            debug!(target: "bundling", "Merging with {} previous bundler inputs", build_state.bundler_inputs.len());

            let current_imports: FxHashSet<String> = bundler_inputs
                .iter()
                .map(|input| input.import.clone())
                .collect();

            // Add previous inputs that aren't in the current set
            for prev_input in &build_state.bundler_inputs {
                if !current_imports.contains(prev_input) {
                    bundler_inputs.push(InputItem {
                        import: prev_input.clone(),
                        name: Some(
                            PathBuf::from(prev_input)
                                .file_stem()
                                .unwrap_or_default()
                                .to_string_lossy()
                                .to_string(),
                        ),
                    });
                }
            }
        }

        debug!(
            target: "bundling",
            "Bundler inputs: {:?}",
            bundler_inputs
                .iter()
                .map(|input| input.import.clone())
                .collect::<Vec<String>>()
        );

        // Store bundler inputs in build state for next incremental build
        if options.incremental {
            build_state.bundler_inputs = bundler_inputs
                .iter()
                .map(|input| input.import.clone())
                .collect();
        }

        if !bundler_inputs.is_empty() {
            let mut module_types_hashmap = FxHashMap::default();
            // Fonts
            module_types_hashmap.insert("woff".to_string(), ModuleType::Asset);
            module_types_hashmap.insert("woff2".to_string(), ModuleType::Asset);
            module_types_hashmap.insert("ttf".to_string(), ModuleType::Asset);
            module_types_hashmap.insert("otf".to_string(), ModuleType::Asset);
            module_types_hashmap.insert("eot".to_string(), ModuleType::Asset);
            // Images
            module_types_hashmap.insert("png".to_string(), ModuleType::Asset);
            module_types_hashmap.insert("jpg".to_string(), ModuleType::Asset);
            module_types_hashmap.insert("jpeg".to_string(), ModuleType::Asset);
            module_types_hashmap.insert("gif".to_string(), ModuleType::Asset);
            module_types_hashmap.insert("svg".to_string(), ModuleType::Asset);
            module_types_hashmap.insert("webp".to_string(), ModuleType::Asset);
            module_types_hashmap.insert("avif".to_string(), ModuleType::Asset);
            module_types_hashmap.insert("ico".to_string(), ModuleType::Asset);

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

            let result = bundler.write().await?;

            // Track transitive dependencies from bundler output
            // For each chunk, map all its modules to the routes that use the entry point
            // For assets (images, fonts via CSS url()), map them to all routes using any entry point
            if options.incremental {
                // First, collect all routes that use any bundler entry point
                let mut all_bundler_routes: FxHashSet<RouteIdentifier> = FxHashSet::default();

                for output in &result.assets {
                    if let Output::Chunk(chunk) = output {
                        // Get the entry point for this chunk
                        if let Some(facade_module_id) = &chunk.facade_module_id {
                            // Try to find routes using this entry point
                            let entry_path = PathBuf::from(facade_module_id.as_str());
                            let canonical_entry = entry_path.canonicalize().ok();

                            // Look up routes for this entry point
                            let routes = canonical_entry
                                .as_ref()
                                .and_then(|p| build_state.asset_to_routes.get(p))
                                .cloned();

                            if let Some(routes) = routes {
                                // Collect routes for asset tracking later
                                all_bundler_routes.extend(routes.iter().cloned());

                                // Register all modules in this chunk as dependencies for those routes
                                let mut transitive_count = 0;
                                for module_id in &chunk.module_ids {
                                    let module_path = PathBuf::from(module_id.as_str());
                                    if let Ok(canonical_module) = module_path.canonicalize() {
                                        // Skip the entry point itself (already tracked)
                                        if Some(&canonical_module) != canonical_entry.as_ref() {
                                            for route in &routes {
                                                build_state.track_asset(
                                                    canonical_module.clone(),
                                                    route.clone(),
                                                );
                                            }
                                            transitive_count += 1;
                                        }
                                    }
                                }
                                if transitive_count > 0 {
                                    debug!(target: "build", "Tracked {} transitive dependencies for {}", transitive_count, facade_module_id);
                                }
                            }
                        }
                    }
                }

                // Now track Output::Asset items (images, fonts, etc. referenced via CSS url() or JS imports)
                // These are mapped to all routes that use any bundler entry point
                if !all_bundler_routes.is_empty() {
                    let mut asset_count = 0;
                    for output in &result.assets {
                        if let Output::Asset(asset) = output {
                            for original_file in &asset.original_file_names {
                                let asset_path = PathBuf::from(original_file);
                                if let Ok(canonical_asset) = asset_path.canonicalize() {
                                    for route in &all_bundler_routes {
                                        build_state
                                            .track_asset(canonical_asset.clone(), route.clone());
                                    }
                                    asset_count += 1;
                                }
                            }
                        }
                    }
                    if asset_count > 0 {
                        debug!(target: "build", "Tracked {} bundler assets for {} routes", asset_count, all_bundler_routes.len());
                    }
                }
            }
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

    // Save build state for next incremental build (only if incremental is enabled)
    if options.incremental {
        if let Err(e) = build_state.save(build_cache_dir) {
            warn!(target: "build", "Failed to save build state: {}", e);
        } else {
            debug!(target: "build", "Build state saved to {}", build_cache_dir.join("build_state.json").display());
        }
    }

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

    trace!(target: "build", "Writing HTML file: {}", file_path.display());
    fs::write(file_path, content)?;

    Ok(())
}
