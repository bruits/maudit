---
title: "Maudit as a library"
description: "Learn how to use Maudit as a library in your Rust projects."
section: "advanced"
---

[Maudit is built as a library, not a framework](/docs/philosophy/#maudit-is-a-library-not-a-framework). It is absolutely primordial to us that Maudit does not feel like this black box that you cannot inspect or understand. You should be able to see how everything works, and make it work for you.

As such, in this guide, we'll be building our own minimal [entrypoint](/docs/entrypoint/) to replace the built-in [`coronate`](https://docs.rs/maudit/latest/maudit/fn.coronate.html) function. This will give us a better understanding of how Maudit works, and allow us to pick apart the different pieces and customize them to our needs.

> The result of this guide is available in the [library example](https://github.com/bruits/maudit/tree/main/examples/library) in the Maudit repository.

## Function signature

The built-in `coronate` function takes a list of routes (which all implements the [FullRoute](https://docs.rs/maudit/latest/maudit/page/trait.FullRoute.html) trait), content sources, and some build options. We'll do the same.

```rs
use maudit::{
  content::ContentSources,
  page::{FullRoute, RouteAssets, RouteContent},
  route::{DynamicRouteContext, PageContext, RouteParams, RouteType},
  BuildOptions,
};

pub fn build_website(
  routes: &[&dyn FullRoute],
  mut content_sources: ContentSources,
  options: BuildOptions
) -> Result<(), Box<dyn std::error::Error>> {
  // We'll fill this in later.
  Ok(())
}
```

`Box<dyn std::error::Error>` is typically seen as an anti-pattern in Rust, as it makes it hard to handle specific error types. But, for the sake of simplicity, we'll use it here.

## Building pages

The first step in building our own entrypoint is to iterate over the routes and build each page. Routes can either be static (i.e. `/index`) or dynamic (i.e. `/articles/[id]`). For now, we'll only handle static routes.

```rs
pub fn build_website(
  routes: &[&dyn FullRoute],
  mut content_sources: ContentSources,
  options: BuildOptions,
) -> Result<(), Box<dyn std::error::Error>> {

  // Options we'll be passing to RouteAssets instances.
  // This value automatically has the paths joined based on the output directory in BuildOptions for us, so we don't have to do it ourselves.
  let page_assets_options = options.page_assets_options();

  for route in routes {
    match route.route_type() {
      RouteType::Static => {
        // Our page does not include content or assets, but we'll set those up for future use.
        let content = RouteContent::new(&content_sources);
        let mut page_assets = RouteAssets::new(&page_assets_options);

        // Static and dynamic routes share the same interface for building, but static routes do not require any parameters.
        // As such, we can just pass an empty set of parameters (the default for RouteParams).
        let params = RouteParams::default();

        // Every page has a PageContext, which contains information about the current route, as well as access to content and assets.
        let url = route.url(&params);
        let mut ctx = PageContext::from_static_route(&content, &mut page_assets, &url);

        let content = route.build(&mut ctx)?;

        let route_filepath = route.file_path(&params, &options.output_dir);

        // On some platforms, creating a file in a nested directory requires that the directory already exists or the file creation will fail.
        if let Some(parent_dir) = route_filepath.parent() {
            fs::create_dir_all(parent_dir)?
        }

        fs::write(route_filepath, content)?;
      }
      RouteType::Dynamic => {
        unimplemented!("We'll handle dynamic routes later");
      }
    }
  }

  Ok(())
}
```

And with just this code, we can already build our first page! Adding a static Maudit page to the routes and running your custom entrypoint will generate the page in the output directory, as expected.

But, if you try to use assets, you'll notice that your pages are pointing to non-existing assets. And similarly, if you try to use content in your page, you'll never be able to get any entries from your sources. Let's fix that!

## Handling assets

Implementing asset processing is a bit outside of the scope of this guide, but we'll at least make sure that assets are working by copying them to the output directory.

This can be done by iterating over the assets registered in `page_assets` and copying them to their build path after having called `route.build()` (which registers the assets used by the page):

```rs
for asset in page_assets.assets() {
    fs::copy(asset.path(), asset.build_path())?;
}
```

And that's it! Now, any asset used in a page will be copied to the output directory when building the page. Onto content.

## Handling content

In the current implementation, trying to use content will result in an empty list of entries. Despite what the syntax might suggest, content sources are not automatically initialized when creating a `ContentSources` instance through the `content_sources![]` macro.

If you've copied the previous snippets, you might have noticed that Rust has been complaining about `content_sources` being mutable but never mutated.

We'll fix that now by initializing each content source by adding the following line before the loop over routes:

```rs
content_sources.init_all();
```

That's all! Now, any content source used in a page will be properly loaded and available for use. This is the most straightforward way to initialize content sources, but a more advanced implementation could for instance initialize content sources in parallel, lazily when a page actually requests content from a source or using advanced caching strategies.

## Dynamic routes

A dynamic route is a route that generates multiple pages based on parameters. For instance, a blog might have a dynamic route `/posts/[slug]`, where `[slug]` is a parameter that can take different values for each blog post.

Each individual page is essentially a static route, but it has a slightly different context available to it.

```rs
// No changes before this block.

RouteType::Dynamic => {
  // The `get_routes` method returns all the possible routes for this page, along with their parameters and properties.
  // It is very common for dynamic pages to be based on content, for instance a blog post page that has one route per blog post.
  // As such, we create essentially a mini `PageContext` through `DynamicRouteContext` that includes the content sources, so that the page can use them to generate its routes.

  // Every page of a route may share a reference to the same RouteContent and RouteAssets instance, as it can help with caching.
  // However, it is not stricly necessary, and you may want to instead create a new instance of RouteAssets especially if you were to parallelize the building of pages.
  let mut page_assets = RouteAssets::new(&page_assets_options);
  let content = RouteContent::new(&content_sources);

  let dynamic_ctx = DynamicRouteContext {
      content: &content,
      assets: &mut page_assets,
  };

  let routes = route.get_routes(&dynamic_ctx);

  for dynamic_route in routes {
      // The dynamic route includes the parameters for this specific route.
      let params = &dynamic_route.0;

      // Here the context is created from a dynamic route, as the context has to include the route parameters and properties.
      let url = route.url(params);
      let mut ctx = PageContext::from_dynamic_route(
          &dynamic_route,
          &content,
          &mut page_assets,
          &url,
      );

      // Everything after this is the same as for static routes.
  }
}
```

## Conclusion

And with that, you've succesfully rebuilt Maudit at home! There's a few more things that can be done to improve this implementation, like adding logging, copying static assets (from `options.static_dir`), asset processing, better error handling, parallelization, caching, etc, etc.

But, this is a fully functional implementation that can be used as a starting point for more advanced use cases... or just as a learning exercise to understand how Maudit works under the hood.
