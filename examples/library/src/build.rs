use std::fs;

use maudit::{
    assets::RouteAssets,
    content::{ContentSources, RouteContent},
    route::{DynamicRouteContext, FullRoute, PageContext, PageParams, RouteType},
    BuildOptions,
};

pub fn build_website(
    routes: &[&dyn FullRoute],
    mut content_sources: ContentSources,
    options: &BuildOptions,
) -> Result<(), Box<dyn std::error::Error>> {
    // Initialize all the content sources;
    content_sources.init_all();

    // Options we'll be passing to RouteAssets instances.
    // This value automatically has the paths joined based on the output directory in BuildOptions for us, so we don't have to do it ourselves.
    let route_assets_options = options.route_assets_options();

    // Create the assets directory if it doesn't exist.
    fs::create_dir_all(&route_assets_options.assets_dir)?;

    for route in routes {
        match route.route_type() {
            RouteType::Static => {
                // Our page does not include content or assets, but we'll set those up for future use.
                let content = RouteContent::new(&content_sources);
                let mut page_assets = RouteAssets::new(&route_assets_options);

                // Static and dynamic routes share the same interface for building, but static routes do not require any parameters.
                // As such, we can just pass an empty set of parameters (the default for PageParams).
                let params = PageParams::default();

                // Every page has a PageContext, which contains information about the current page, as well as access to content and assets.
                let url = route.url(&params);
                let mut ctx = PageContext::from_static_route(
                    &content,
                    &mut page_assets,
                    &url,
                    &options.base_url,
                );

                let content = route.build(&mut ctx)?;

                let page_filepath = route.file_path(&params, &options.output_dir);

                // On some platforms, creating a file in a nested directory requires that the directory already exists or the file creation will fail.
                if let Some(parent_dir) = page_filepath.parent() {
                    fs::create_dir_all(parent_dir)?
                }

                fs::write(page_filepath, content)?;

                // Copy all assets used by this page.
                for asset in page_assets.assets() {
                    fs::copy(asset.path(), asset.build_path())?;
                }
            }
            RouteType::Dynamic => {
                // The `get_pages` method returns all the possible pages for this route, along with their parameters and properties.
                // It is very common for dynamic pages to be based on content, for instance a blog post page that has one route per blog post.
                // As such, we create a mini PageContext that includes the content sources, so that the route can use them to generate its pages.

                // Every page of a route may share a reference to the same RouteContent and RouteAssets instance, as it can help with caching.
                // However, it is not stricly necessary, and you may want to instead create a new instance of RouteAssets especially if you were to parallelize the building of pages.
                let content = RouteContent::new(&content_sources);
                let mut page_assets = RouteAssets::new(&route_assets_options);

                let mut dynamic_ctx = DynamicRouteContext {
                    content: &content,
                    assets: &mut page_assets,
                };

                let routes = route.get_pages(&mut dynamic_ctx);

                let content = RouteContent::new(&content_sources);

                for page in routes {
                    // The dynamic route includes the parameters for this specific page.
                    let params = &page.0;

                    // Here the context is created from a dynamic route, as the context has to include the page parameters and properties.
                    let url = route.url(params);
                    let mut ctx = PageContext::from_dynamic_route(
                        &page,
                        &content,
                        &mut page_assets,
                        &url,
                        &options.base_url,
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
