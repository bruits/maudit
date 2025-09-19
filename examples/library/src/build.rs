use std::fs;

use maudit::{
    assets::PageAssets,
    content::{ContentSources, PageContent},
    page::{DynamicRouteContext, FullPage, RouteContext, RouteParams, RouteType},
    BuildOptions,
};

pub fn build_website(
    routes: &[&dyn FullPage],
    mut content_sources: ContentSources,
    options: BuildOptions,
) -> Result<(), Box<dyn std::error::Error>> {
    // Initialize all the content sources;
    content_sources.init_all();

    // Options we'll be passing to PageAssets instances.
    // This value automatically has the paths joined based on the output directory in BuildOptions for us, so we don't have to do it ourselves.
    let page_assets_options = options.page_assets_options();

    // Create the assets directory if it doesn't exist.
    fs::create_dir_all(&page_assets_options.assets_dir)?;

    for route in routes {
        match route.route_type() {
            RouteType::Static => {
                // Our page does not include content or assets, but we'll set those up for future use.
                let content = PageContent::new(&content_sources);
                let mut page_assets = PageAssets::new(&page_assets_options);

                // Static and dynamic routes share the same interface for building, but static routes do not require any parameters.
                // As such, we can just pass an empty set of parameters (the default for RouteParams).
                let params = RouteParams::default();

                // Every page has a RouteContext, which contains information about the current route, as well as access to content and assets.
                let url = route.url(&params);
                let mut ctx = RouteContext::from_static_route(&content, &mut page_assets, &url);

                let content = route.build(&mut ctx)?;

                let route_filepath = route.file_path(&params, &options.output_dir);

                // On some platforms, creating a file in a nested directory requires that the directory already exists or the file creation will fail.
                if let Some(parent_dir) = route_filepath.parent() {
                    fs::create_dir_all(parent_dir)?
                }

                fs::write(route_filepath, content)?;

                // Copy all assets used by this page.
                for asset in page_assets.assets() {
                    fs::copy(asset.path(), asset.build_path())?;
                }
            }
            RouteType::Dynamic => {
                // The `get_routes` method returns all the possible routes for this page, along with their parameters and properties.
                // It is very common for dynamic pages to be based on content, for instance a blog post page that has one route per blog post.
                // As such, we create a mini RouteContext that includes the content sources, so that the page can use them to generate its routes.

                let dynamic_ctx = DynamicRouteContext {
                    content: &PageContent::new(&content_sources),
                };

                let routes = route.get_routes(&dynamic_ctx);

                // Every page can share a reference to the same PageContent instance, as it is just a view into the content sources.
                let content = PageContent::new(&content_sources);

                for dynamic_route in routes {
                    // However, since page assets is a mutable structure that tracks which assets have been used, we need a new instance for each route.
                    // This is especially relevant if we were to parallelize this loop in the future.
                    let mut page_assets = PageAssets::new(&page_assets_options);

                    // The dynamic route includes the parameters for this specific route.
                    let params = &dynamic_route.0;

                    // Here the context is created from a dynamic route, as the context has to include the route parameters and properties.
                    let url = route.url(params);
                    let mut ctx = RouteContext::from_dynamic_route(
                        &dynamic_route,
                        &content,
                        &mut page_assets,
                        &url,
                    );

                    // Everything below here is the same as for static routes.

                    let content = route.build(&mut ctx)?;

                    let route_filepath = route.file_path(params, &options.output_dir);

                    if let Some(parent_dir) = route_filepath.parent() {
                        fs::create_dir_all(parent_dir)?
                    }

                    fs::write(route_filepath, content)?;

                    for asset in page_assets.assets() {
                        fs::copy(asset.path(), asset.build_path())?;
                    }
                }
            }
        }
    }

    Ok(())
}
