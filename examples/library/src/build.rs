use std::path::PathBuf;
use std::str::FromStr;
use std::{collections::HashSet, fs};

use maudit::page::DynamicRouteContext;
use maudit::{
    assets::PageAssets,
    content::{ContentSources, PageContent},
    page::{FullPage, RouteContext, RouteParams, RouteType},
    BuildOptions,
};

pub fn build_website(
    routes: &[&dyn FullPage],
    mut content_sources: ContentSources,
    options: BuildOptions,
) -> Result<(), Box<dyn std::error::Error>> {
    let dist_dir = PathBuf::from_str(&options.output_dir)?;

    // Initialize all the content sources;
    content_sources.init_all();

    let mut all_assets: HashSet<(PathBuf, PathBuf)> = HashSet::new();

    for route in routes {
        match route.route_type() {
            RouteType::Static => {
                // Our page does not include content or assets, but we'll set those up for future use.
                let content = PageContent::new(&content_sources);
                let mut page_assets = PageAssets::new(options.assets_dir.clone().into());

                // Static and dynamic routes share the same interface for building, but static routes do not require any parameters.
                // As such, we can just pass an empty set of parameters (the default for RouteParams).
                let params = RouteParams::default();

                // Every page has a RouteContext, which contains information about the current route, as well as access to content and assets.
                let mut ctx = RouteContext::from_static_route(
                    &content,
                    &mut page_assets,
                    route.url(&params).clone(),
                );

                let content = route.build(&mut ctx)?;

                // FullPage.file_path() returns a path that does not include the output directory, so we need to join it with dist_dir.
                let final_filepath = dist_dir.join(route.file_path(&params));

                // On some platforms, creating a file in a nested directory requires that the directory already exists.
                if let Some(parent_dir) = final_filepath.parent() {
                    fs::create_dir_all(parent_dir)?
                }

                fs::write(final_filepath, content)?;

                // Collect all assets used by this page.
                all_assets.extend(page_assets.assets().map(|asset| {
                    (
                        dist_dir.join(asset.build_path()),
                        asset.path().to_path_buf(),
                    )
                }));
            }
            RouteType::Dynamic => {
                // The `routes` method returns all the possible routes for this page, along with their parameters and properties.
                // It is very common for dynamic pages to be based on content, for instance a blog post page that has one route per blog post.
                // As such, we create a mini RouteContext that includes the content sources, so that the page can use them to generate its routes.

                let dynamic_ctx = DynamicRouteContext {
                    content: &PageContent::new(&content_sources),
                };

                let routes = route.routes_internal(&dynamic_ctx);

                // Every page can share the same PageContent instance, as it is just a view into the content sources.
                let content = PageContent::new(&content_sources);

                for dynamic_route in routes {
                    // However, since page assets is a mutable structure that tracks which assets have been used, we need a new instance for each route.
                    // This is especially relevant if we were to parallelize this loop in the future.
                    let mut page_assets = PageAssets::new(options.assets_dir.clone().into());

                    // The dynamic route includes the parameters for this specific route.
                    let params = &dynamic_route.0;

                    // Here the context is created from a dynamic route, as the context has to include the route parameters and properties.
                    let mut ctx = RouteContext::from_dynamic_route(
                        &dynamic_route,
                        &content,
                        &mut page_assets,
                        route.url(params),
                    );

                    // Everything from here is the same as for static routes.
                    let content = route.build(&mut ctx)?;

                    let final_file_path = &dist_dir.join(route.file_path(params));

                    if let Some(parent_dir) = final_file_path.parent() {
                        fs::create_dir_all(parent_dir)?
                    }

                    fs::write(final_file_path, content)?;

                    // Collect all assets used by this page.
                    all_assets.extend(page_assets.assets().map(|asset| {
                        (
                            dist_dir.join(asset.build_path()),
                            asset.path().to_path_buf(),
                        )
                    }));
                }
            }
        }
    }

    // Copy all assets to the output directory.
    for (dest_path, src_path) in all_assets {
        if let Some(parent) = dest_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(src_path, dest_path)?;
    }

    Ok(())
}
