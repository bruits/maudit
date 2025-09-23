---
title: "Routing"
description: "How to create pages and routes in Maudit"
section: "core-concepts"
---

## Registering Routes

Routes must be passed to the `coronate` function in [the entrypoint](/docs/entrypoint) in order to be built.

The first argument to the `coronate` function is a `Vec` of all the routes that should be built. This list can be created using the `routes!` macro to make it more concise.

```rs
use routes::Index;
use maudit::{coronate, routes, BuildOptions, BuildOutput};

fn main() -> Result<BuildOutput, Box<dyn std::error::Error>> {
    coronate(
      routes![Index],
      vec![].into(),
      BuildOptions::default()
    )
}
```

## Static Routes

To create a new page in your Maudit project, create a struct and implement the `Route` trait for it, adding the `#[route]` attribute to the struct definition with the path of the route as an argument. The path can be any Rust expression, as long as its value can be converted to String. (i.e. `.to_string()` will be called on it)

```rs
use maudit::route::prelude::*;

#[route("/hello-world")]
pub struct HelloWorld;

impl Route for HelloWorld {
  fn render(&self, ctx: &mut PageContext) -> impl Into<RenderResult> {
    "Hello, world!"
  }
}
```

The `Route` trait requires the implementation of a `render` method that returns any types that can be converted into `RenderResult`. This method is called when the page is built and should return the content that will be displayed. In most cases, you'll be using a templating library to create HTML content.

Maudit implements `Into<RenderResult>` for the following types:

- `String`, `Vec<u8>`, `&str`, `&[u8]`
- `Result<T, E> where T: Into<RenderResult> and E: std::error::Error` (see [Handling Errors](#handling-errors) for more information)
- [Various templating libraries](/docs/templating/)

Finally, make sure to [register the page](#registering-routes) in the `coronate` function for it to be built.

## Dynamic Routes

Maudit supports creating dynamic routes with parameters. Allowing one to create many pages that share the same structure and logic, but with different content. For example, a blog where each post has a unique URL, e.g., `/posts/my-blog-post`.

To create a dynamic route, export a struct using the `route` attribute and add parameters by enclosing them in square brackets (ex: `/posts/[slug]`) in the route's path.

In addition to the `render` method, dynamic routes must implement a `pages` method for Route. The `pages` method returns a list of all the possible values for each parameter in the route's path, so that Maudit can generate all the necessary pages.

```rs
use maudit::route::prelude::*;

#[route("/posts/[slug]")]
pub struct Post;

#[derive(Params, Clone)]
pub struct Params {
  pub slug: String,
}

impl Route<Params> for Post {
  fn render(&self, ctx: &mut PageContext) -> impl Into<RenderResult> {
    let params = ctx.params::<Params>();

    format!("Hello, {}!", params.slug)
  }

  fn pages(&self, ctx: &mut DynamicRouteContext) -> Pages<Params> {
    vec![Page::from_params(Params {
      slug: "hello-world".to_string(),
    })]
  }
}
```

The route parameters are automatically extracted from the URL and made available through the `ctx.params::<T>()` method in the `PageContext` struct, providing type-safe access to the values.

```rs
use maudit::route::prelude::*;

#[route("/posts/[slug]")]
pub struct Post;

#[derive(Params, Clone)]
pub struct Params {
  pub slug: String,
}

impl Route for Post {
  fn render(&self, ctx: &mut PageContext) -> String {
    let slug = ctx.params::<Params>().slug;
    format!("Hello, {}!", slug)
  }

  fn pages(&self, ctx: &mut DynamicRouteContext) -> Pages<Params> {
    vec![Page::from_params(Params {
      slug: "hello-world".to_string(),
    })]
  }
}
```

The struct used for the parameters must implement `Into<PageParams>`, which can be done automatically by deriving the `Params` trait. The fields of the struct must implement the `Display` trait, as they will be converted to strings to be used in the final URLs and file paths.

Like static routes, dynamic routes must be [registered](#registering-routes) in the `coronate` function in order for them to be built.

### Optional parameters

Dynamic routes can also have optional parameters by using the `Option<T>` type in the parameters struct. These parameters will be completely removed from the URL and file path when they are `None`. For instance, in a route with the path `/posts/[category]/[slug]`, if the `category` parameter is `None`, the resulting URL will be `/posts/my-blog-post/`.

This feature is notably useful when creating paginated routes (ex: `/posts/[page]`), where the first page sometimes does not include a page number in the URL, but subsequent pages do (e.g., `/blog` for the first page and `/blog/1` for the second page).

Maudit will automatically collapse repeated slashes in the URL and file path into a single slash, as such `/articles/[slug]/[page]/` where `page` is `None` will result in `/articles/my-article/`, and not `/articles/my-article//`.

## Endpoints

Maudit supports returning other types of content besides HTML, such as JSON, plain text or binary data. To do this, add a file extension to the route path and return the content in the `render` method. Both static and dynamic routes can be used as endpoints.

```rs
use maudit::route::prelude::*;

#[route("/api.json")]
pub struct HelloWorldJson;

impl Route for HelloWorldJson {
  fn render(&self, ctx: &mut PageContext) -> impl Into<RenderResult> {
    r#"{"message": "Hello, world!"}"#
  }
}
```

Endpoints must also be [registered](#registering-routes) in the `coronate` function in order for them to be built.

## Handling Errors

Maudit implements `Into<RenderResult>` for `Result<T: Into<RenderResult>, E: std::error::Error>`. This allows you to use the `?` operator in your `render` method to ergonomically propagate errors that may occur during rendering without needing to change the function's signature.

The error will be propagated all the way to [`coronate()`](https://docs.rs/maudit/latest/maudit/fn.coronate.html), which will return an error if any page fails to render.

```rs
impl Route for HelloWorld {
  fn render(&self, ctx: &mut PageContext) -> impl Into<RenderResult> {
    some_operation_that_might_fail()?;

    Ok("Hello, world!")
  }
}
```
