use core::panic;
use std::{
    env,
    fs::{self, File, remove_dir_all},
    io::{self, Write},
    path::{Path, PathBuf},
    process::Command,
    str::FromStr,
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

use crate::{
    BuildOptions, BuildOutput,
    assets::{self},
    content::{Content, ContentSources},
    errors::BuildError,
    logging::print_title,
    page::{DynamicRouteContext, FullPage, RenderResult, RouteContext, RouteParams, RouteType},
    route::{
        ParameterDef, extract_params_from_raw_route, get_route_file_path,
        get_route_type_from_route_params, get_route_url,
    },
};
use colored::{ColoredString, Colorize};
use log::{info, trace};
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
    tailwind_path: String,
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
            let start_tailwind = SystemTime::now();
            let tailwind_output =
                Command::new(&self.tailwind_path)
                    .args(["--input", args.id])
                    .arg("--minify") // TODO: Allow disabling minification
                    .arg("--map") // TODO: Allow disabling source maps
                    .output()
                    .unwrap_or_else(|e| {
                        // TODO: Return a proper error instead of panicking
                        panic!(
                            "Failed to execute Tailwind CSS command, is it installed and is the path to its binary correct?\nCommand: '{}', Args: ['--input', '{}', '--minify', '--map']. Error: {}",
                            &self.tailwind_path,
                            args.id,
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

            info!("Tailwind took {:?}", start_tailwind.elapsed().unwrap());

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

    let old_dist_tmp_dir = if options.clean_output_dir {
        let duration = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        let num = (duration.as_secs() + duration.subsec_nanos() as u64) % 100000;
        let new_dir_for_old_dist = env::temp_dir().join(format!("maudit_old_dist_{}", num));
        let _ = fs::rename(&dist_dir, &new_dir_for_old_dist);
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

    fs::create_dir_all(&dist_dir)?;
    fs::create_dir_all(&assets_dir)?;

    info!(target: "build", "Output directory: {}", dist_dir.to_string_lossy());

    let content_sources_start = SystemTime::now();
    print_title("initializing content sources");
    content_sources.0.iter_mut().for_each(|source| {
        let source_start = SystemTime::now();
        source.init();

        info!(target: "content", "{} initialized in {}", source.get_name(), format_elapsed_time(source_start.elapsed(), &FormatElapsedTimeOptions::default()).unwrap());
    });

    info!(target: "content", "{}", format!("Content sources initialized in {}", format_elapsed_time(
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

    #[allow(clippy::mutable_key_type)] // Image's Hash does not depend on mutable fields
    let build_pages_images: FxHashSet<assets::Image> = FxHashSet::default();
    let build_pages_scripts: FxHashSet<assets::Script> = FxHashSet::default();
    let build_pages_styles: FxHashSet<assets::Style> = FxHashSet::default();

    let page_count = 0;

    // First, collect all the individual pages to render
    let mut all_page_jobs: Vec<(
        &dyn FullPage,
        Vec<ParameterDef>,
        RouteParams,
        Option<Box<dyn std::any::Any + Send + Sync>>,
        Option<Box<dyn std::any::Any + Send + Sync>>,
        bool, // is_dynamic
    )> = Vec::new();

    // Collect static and dynamic pages
    for route in routes {
        let params_def = extract_params_from_raw_route(&route.route_raw());
        let route_type = get_route_type_from_route_params(&params_def);
        match route_type {
            RouteType::Static => {
                let params = RouteParams(FxHashMap::default());
                all_page_jobs.push((*route, params_def, params, None, None, false));
            }
            RouteType::Dynamic => {
                let mut dynamic_content = Content::new(&content_sources.0);
                let mut dynamic_route_context = DynamicRouteContext {
                    content: &mut dynamic_content,
                };

                let routes_data = route.routes_internal(&mut dynamic_route_context);

                if routes_data.is_empty() {
                    info!(target: "build", "{} is a dynamic route, but its implementation of Page::routes returned an empty Vec. No pages will be generated for this route.", route.route_raw().to_string().bold());
                    continue;
                } else {
                    info!(target: "build", "{}", route.route_raw().to_string().bold());
                }

                for (params, typed_params, props) in routes_data {
                    all_page_jobs.push((
                        *route,
                        params_def.clone(),
                        params,
                        Some(typed_params),
                        Some(props),
                        true,
                    ));
                }
            }
        }
    }

    // Wrap shared data in mutexes for parallel access
    let build_pages_images = Mutex::new(build_pages_images);
    let build_pages_scripts = Mutex::new(build_pages_scripts);
    let build_pages_styles = Mutex::new(build_pages_styles);
    let build_metadata_mutex = Mutex::new(&mut build_metadata);
    let page_count_mutex = Mutex::new(page_count);

    // Now render all pages in parallel
    let results: Vec<Result<(), String>> = all_page_jobs
        .par_iter()
        .map(|(route, params_def, params, typed_params_opt, props_opt, is_dynamic)| -> Result<(), String> {
            let route_start = SystemTime::now();
            let mut page_assets = assets::PageAssets {
                assets_dir: options.assets_dir.clone().into(),
                ..Default::default()
            };

            let mut content = Content::new(&content_sources.0);
            let mut ctx = RouteContext {
                raw_params: params,
                content: &mut content,
                assets: &mut page_assets,
                current_url: get_route_url(&route.route_raw(), params_def, params),
                params: typed_params_opt.as_ref().map(|p| p.as_ref()).unwrap_or(&()),
                props: props_opt.as_ref().map(|p| p.as_ref()).unwrap_or(&()),
            };

            let (file_path, mut file) = create_route_file(*route, params_def, params, &dist_dir)
                .map_err(|e| format!("Failed to create route file: {}", e))?;

            let result = route.render_internal(&mut ctx);

            finish_route(
                result,
                &mut file,
                &page_assets.included_styles,
                &page_assets.included_scripts,
                route.route_raw(),
            ).map_err(|e| format!("Failed to finish route: {}", e))?;

            // Thread-safe updates using mutexes
            {
                let mut images = build_pages_images.lock().unwrap();
                images.extend(page_assets.images);
            }
            {
                let mut scripts = build_pages_scripts.lock().unwrap();
                scripts.extend(page_assets.scripts);
            }
            {
                let mut styles = build_pages_styles.lock().unwrap();
                styles.extend(page_assets.styles);
            }
            {
                let mut metadata = build_metadata_mutex.lock().unwrap();
                metadata.add_page(
                    route.route_raw().to_string(),
                    file_path.to_string_lossy().to_string(),
                    if typed_params_opt.is_some() { Some(params.0.clone()) } else { None },
                );
            }
            {
                let mut count = page_count_mutex.lock().unwrap();
                *count += 1;
            }

            let display_format = if *is_dynamic {
                format!("├─ {} {}", file_path.to_string_lossy().dimmed(), format_elapsed_time(route_start.elapsed(), &route_format_options).unwrap())
            } else {
                format!("{} -> {} {}", get_route_url(&route.route_raw(), params_def, params), file_path.to_string_lossy().dimmed(), format_elapsed_time(route_start.elapsed(), &route_format_options).unwrap())
            };
            info!(target: "pages", "{}", display_format);

            Ok(())
        })
        .collect();

    // Handle any errors from parallel processing
    for result in results {
        if let Err(e) = result {
            return Err(e.into());
        }
    }

    // Extract data from mutexes
    let final_page_count = *page_count_mutex.lock().unwrap();
    let final_build_pages_images = build_pages_images.into_inner().unwrap();
    let final_build_pages_scripts = build_pages_scripts.into_inner().unwrap();
    let final_build_pages_styles = build_pages_styles.into_inner().unwrap();
    info!(target: "pages", "{}", format!("generated {} pages in {}", final_page_count, format_elapsed_time(pages_start.elapsed(), &section_format_options).unwrap()).bold());

    if !final_build_pages_styles.is_empty() || !final_build_pages_scripts.is_empty() {
        let assets_start = SystemTime::now();
        print_title("generating assets");

        let css_inputs = final_build_pages_styles
            .iter()
            .map(|style| InputItem {
                name: Some(
                    style
                        .final_file_name()
                        .strip_suffix(&format!(
                            ".{}",
                            style
                                .path()
                                .extension()
                                .map(|ext| ext.to_str().unwrap())
                                .unwrap_or("")
                        ))
                        .unwrap_or(&style.final_file_name())
                        .to_string(),
                ),
                import: { style.path().to_string_lossy().to_string() },
            })
            .collect::<Vec<InputItem>>();

        let bundler_inputs = final_build_pages_scripts
            .iter()
            .map(|script| InputItem {
                import: script.path().to_string_lossy().to_string(),
                name: Some(
                    script
                        .final_file_name()
                        .strip_suffix(&format!(".{}", script.final_extension()))
                        .unwrap_or(&script.final_file_name())
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
                    minify: Some(rolldown::RawMinifyOptions::Bool(true)),
                    dir: Some(assets_dir.to_string_lossy().to_string()),
                    module_types: Some(module_types_hashmap),

                    ..Default::default()
                },
                vec![Arc::new(TailwindPlugin {
                    tailwind_path: options.tailwind_binary_path.clone(),
                    tailwind_entries: final_build_pages_styles
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

        info!(target: "build", "{}", format!("Assets generated in {}", format_elapsed_time(assets_start.elapsed(), &section_format_options).unwrap()).bold());
    }

    if !final_build_pages_images.is_empty() {
        print_title("processing images");

        let start_time = SystemTime::now();
        final_build_pages_images.par_iter().for_each(|image| {
            let start_process = SystemTime::now();
            let dest_path = assets_dir.join(image.final_file_name());
            if let Some(image_options) = &image.options {
                images::process_image(image, &dest_path, image_options);
            } else if !dest_path.exists() {
                // TODO: Check if copying should be done in this parallel iterator, I/O doesn't benefit from parallelism so having those tasks here might just be slowing processing
                fs::copy(image.path(), &dest_path).unwrap_or_else(|e| {
                    panic!(
                        "Failed to copy image from {} to {}: {}",
                        image.path().to_string_lossy(),
                        dest_path.to_string_lossy(),
                        e
                    )
                });
            }
            info!(target: "assets", "{} -> {} {}", image.path().to_string_lossy(), dest_path.to_string_lossy().dimmed(), format_elapsed_time(start_process.elapsed(), &route_format_options).unwrap().dimmed());
        });

        info!(target: "assets", "{}", format!("Images processed in {}", format_elapsed_time(start_time.elapsed(), &section_format_options).unwrap()).bold());
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
