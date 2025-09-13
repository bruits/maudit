use core::panic;
use std::{
    env,
    fs::{self, File, remove_dir_all},
    io::{self, Write},
    path::{Path, PathBuf},
    process::Command,
    str::FromStr,
    sync::Arc,
    time::{Instant, SystemTime, UNIX_EPOCH},
};

use crate::{
    BuildOptions, BuildOutput,
    assets::{self},
    content::{Content, ContentSources},
    errors::BuildError,
    is_dev,
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
                            &self.tailwind_path,
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
    let build_start = Instant::now();
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

    let content_sources_start = Instant::now();
    print_title("initializing content sources");
    content_sources.0.iter_mut().for_each(|source| {
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

    #[allow(clippy::mutable_key_type)] // Image's Hash does not depend on mutable fields
    let mut build_pages_images: FxHashSet<assets::Image> = FxHashSet::default();
    let mut build_pages_scripts: FxHashSet<assets::Script> = FxHashSet::default();
    let mut build_pages_styles: FxHashSet<assets::Style> = FxHashSet::default();

    let mut page_count = 0;

    // TODO: This is fully serial. Parallelizing it is trivial with Rayon and stuff, but it doesn't necessarily make it
    // faster in all cases, making it sometimes even slower due to the overhead. It'd be great to investigate and benchmark
    // this.
    for route in routes {
        let params_def = extract_params_from_raw_route(&route.route_raw());
        let route_type = get_route_type_from_route_params(&params_def);
        match route_type {
            RouteType::Static => {
                let route_start = Instant::now();
                let mut page_assets = assets::PageAssets {
                    assets_dir: options.assets_dir.clone().into(),
                    ..Default::default()
                };

                let params = RouteParams(FxHashMap::default());

                let mut content = Content::new(&content_sources.0);
                let mut ctx = RouteContext {
                    raw_params: &params,
                    content: &mut content,
                    assets: &mut page_assets,
                    current_url: get_route_url(&route.route_raw(), &params_def, &params),

                    // Static routes have no params or props
                    params: &(),
                    props: &(),
                };

                let (file_path, mut file) =
                    create_route_file(*route, &params_def, &params, &dist_dir)?;
                let result = route.render_internal(&mut ctx);

                finish_route(
                    result,
                    &mut file,
                    &page_assets.included_styles,
                    &page_assets.included_scripts,
                    route.route_raw(),
                )?;

                info!(target: "pages", "{} -> {} {}", get_route_url(&route.route_raw(), &params_def, &params), file_path.to_string_lossy().dimmed(), format_elapsed_time(route_start.elapsed(), &route_format_options));

                build_pages_images.extend(page_assets.images);
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

                for (params, typed_params, props) in routes {
                    let mut pages_assets = assets::PageAssets {
                        assets_dir: options.assets_dir.clone().into(),
                        ..Default::default()
                    };
                    let route_start = Instant::now();
                    let mut content = Content::new(&content_sources.0);
                    let mut ctx = RouteContext {
                        raw_params: &params,
                        params: typed_params.as_ref(),
                        props: props.as_ref(),
                        content: &mut content,
                        assets: &mut pages_assets,
                        current_url: get_route_url(&route.route_raw(), &params_def, &params),
                    };

                    let (file_path, mut file) =
                        create_route_file(*route, &params_def, &params, &dist_dir)?;

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

                    info!(target: "pages", "├─ {} {}", file_path.to_string_lossy().dimmed(), format_elapsed_time(route_start.elapsed(), &route_format_options));

                    build_pages_images.extend(pages_assets.images);
                    build_pages_scripts.extend(pages_assets.scripts);
                    build_pages_styles.extend(pages_assets.styles);

                    page_count += 1;
                }
            }
        }
    }

    info!(target: "pages", "{}", format!("generated {} pages in {}", page_count,  format_elapsed_time(pages_start.elapsed(), &section_format_options)).bold());

    if !build_pages_styles.is_empty() || !build_pages_scripts.is_empty() {
        let assets_start = Instant::now();
        print_title("generating assets");

        let css_inputs = build_pages_styles
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

        let bundler_inputs = build_pages_scripts
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
                    minify: Some(rolldown::RawMinifyOptions::Bool(!is_dev())),
                    dir: Some(assets_dir.to_string_lossy().to_string()),
                    module_types: Some(module_types_hashmap),

                    ..Default::default()
                },
                vec![Arc::new(TailwindPlugin {
                    tailwind_path: options.tailwind_binary_path.clone(),
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

        let start_time = Instant::now();
        build_pages_images.par_iter().for_each(|image| {
            let start_process = Instant::now();
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
            info!(target: "assets", "{} -> {} {}", image.path().to_string_lossy(), dest_path.to_string_lossy().dimmed(), format_elapsed_time(start_process.elapsed(), &route_format_options).dimmed());
        });

        info!(target: "assets", "{}", format!("Images processed in {}", format_elapsed_time(start_time.elapsed(), &section_format_options)).bold());
    }

    // Check if static directory exists
    if static_dir.exists() {
        let assets_start = Instant::now();
        print_title("copying assets");

        // Copy the static directory to the dist directory
        copy_recursively(&static_dir, &dist_dir, &mut build_metadata)?;

        info!(target: "build", "{}", format!("Assets copied in {}", format_elapsed_time(assets_start.elapsed(), &FormatElapsedTimeOptions::default())).bold());
    }

    // Remove temporary files
    let _ = remove_dir_all(&tmp_dir);

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
