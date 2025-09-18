---
title: "Maudit as a library"
description: "Learn how to use Maudit as a library in your Rust projects."
section: "advanced"
---

[Maudit is built as a library, not a framework](/docs/philosophy/#maudit-is-a-library-not-a-framework). It is absolutely primordial to us that Maudit does not feel like this black box that you cannot inspect or understand. You should be able to see how everything works, and make it work for you.

As such, in this guide, we'll be building our own minimal [entrypoint](/docs/entrypoint/) to replace the built-in [`coronate`](https://docs.rs/maudit/latest/maudit/fn.coronate.html) function. This will give us a better understanding of how Maudit works, and allow us to customize it to our needs.

> The result of this guide is available in the [library example](https://github.com/bruits/maudit/tree/main/examples/library) in the Maudit repository.

## Setting up the project

We'll start by creating a new Rust project with Maudit as a dependency:

```bash
cargo new library --bin
cd library
cargo add maudit
```

and we'll create a simple Maudit page in `src/pages/index.rs`:

```rs
use maudit::page::prelude::*;

#[route("/")]
pub struct Index;

impl Page for Index {
  fn render(&self, _: &mut RouteContext) -> RenderResult {
    "Hello, Maudit!".into()
  }
}
```

We'll now start building our own entrypoint in `src/build.rs`, which will contain a `build_website` function, taking the same parameters as `coronate`:

```rs
use maudit::page::FullPage;
use maudit::{content::ContentSources, BuildOptions};

pub fn build_website(
  routes: &[&dyn FullPage],
  mut content_sources: ContentSources,
  options: BuildOptions,
) -> Result<(), Box<dyn std::error::Error>> {
  // Implementation will go here

  Ok(())
}
```

Finally, we'll modify `src/main.rs` to call our `build_website` function:

```rs
mod build;

mod pages {
  mod index;
  pub use index::Index;
}

fn main() {
  let _ = build_website(
    routes![Index],
    content_sources![],
    BuildOptions::default(),
  );
}
```

Now that we have our project set up, let's implement the `build_website` function step by step.

## Building pages

The first step in building our own entrypoint is to iterate over the routes and build each page. Routes can either be static (i.e. `/index`) or dynamic (i.e. `/articles/[id]`). For now, we'll only handle static routes.

```rs
use std::fs;
use std::path::PathBuf;
use std::str::FromStr;

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

        // On some platforms, creating a file in a nested directory requires that the directory already exists or `fs::write` will fail.
        if let Some(parent_dir) = final_filepath.parent() {
          fs::create_dir_all(parent_dir)?
        }

        fs::write(final_filepath, content)?;
      }
      RouteType::Dynamic => {
        unimplemented!("We'll handle dynamic routes later");
      }
    }
  }

  Ok(())
}
```

And with just this code, we can already build our first page! Running `cargo run` should create a `dist/index.html` file with the content `Hello, Maudit!`. But, if you try to use assets, you'll notice that they are not copied to the output directory. And similarly, if you try to use content, you'll notice that it'll always throw an error saying that the content source is not found. Let's fix that!

## Handling assets

We won't be implementing any asset processing (like bundling or minification) in this guide, but we'll be implementing the logic to copy assets from the assets directory to the output directory.

```rs
pub fn build_website(
    routes: &[&dyn FullPage],
    mut content_sources: ContentSources,
    options: BuildOptions,
) -> Result<(), Box<dyn std::error::Error>> {
    let dist_dir = PathBuf::from_str(&options.output_dir)?;

    let mut all_assets: HashSet<(PathBuf, PathBuf)> = HashSet::new();

    for route in routes {
        match route.route_type() {
            RouteType::Static => {
                // ... Same as before ...

                // Collect all assets used by this page.
                all_assets.extend(page_assets.assets().map(|asset| {
                    (
                        dist_dir.join(asset.build_path()),
                        asset.path().to_path_buf(),
                    )
                }));
            }
            RouteType::Dynamic => {
                unimplemented!("We'll handle dynamic routes later");
            }
        }
    }

    // Copy all assets to the output directory.
    for (dest_path, src_path) in all_assets {
        // Similar to pages, we need to ensure the parent directory exists or `fs::copy` will fail.
        if let Some(parent) = dest_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(src_path, dest_path)?;
    }

    Ok(())
}
```

This is enough to get us started with assets. Any pages using assets will now have them copied to the output directory. This is a basic implementation, but it is all we need to get assets working. `route.build` already takes care of automatically including scripts and styles in the page, so we don't need to do anything special for that. Additionally, `asset.build_path()` already takes care of adding hashes to filenames in a performant way, so we don't need to worry about that either.

## Handling content

As said previously, our current implementation does not handle content sources. Currently any pages trying to use content would panic with an error saying the requested content source does not exist. To fix this, we need to make sure that the content sources are properly loaded before building the pages.

If you've copied the previous snippets, you might have noticed that Rust has been complaining about `content_sources` being mutable but never mutated. We'll fix that now by initializing each content source before building the pages:

```rs
content_sources.init_all();
```

That's all. Now, any content source used in a page will be properly loaded and available for use. This is the most straightforward way to initialize content sources, but a more advanced implementation could for instance initialize content sources in parallel, lazily when a page actually requests content from a source or using advanced caching strategies.

## Dynamic routes

A dynamic route is a route that generates multiple pages based on some parameters. For instance, a blog post page might have a dynamic route `/posts/[id]`, where `[id]` is a parameter that can take different values for each blog post. Each individual page is essentially a static route, but it has a slightly different context available to it.

```rs
for route in routes {
  match route.route_type() {
    RouteType::Static => {
      // No changes here, same as before.
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

          // Everything after this is the same as for static routes, making sure to use the route parameters when getting the file path.
      }
    }
  }
}
```

And with that, you've succesfully rebuilt Maudit at home! There's a few more things that can be done to improve this implementation, like adding logging, copying static assets, asset processing, better error handling, parallelization, caching, etc, etc. But, this is a fully functional implementation that can be used as a starting point for more advanced use cases... or just as a learning exercise to understand how Maudit works under the hood.
