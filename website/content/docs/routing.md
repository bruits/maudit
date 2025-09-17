---
title: "Routing"
description: "How to create pages and routes in Maudit"
section: "core-concepts"
---

### Static Routes

To create a new page in your Maudit project, create a struct and implement the `Page` trait for it, adding the `#[route]` attribute to the struct definition with the path of the route as an argument. The path can be any Rust expression, as long as it returns a `String`.

```rs
use maudit::page::prelude::*;

#[route("/hello-world")]
pub struct HelloWorld;

impl Page for HelloWorld {
  fn render(&self, ctx: &mut RouteContext) -> RenderResult {
    RenderResult::Text("Hello, world!".to_string())
  }
}
```

The `Page` trait requires the implementation of a `render` method that returns a `RenderResult`. This method is called when the page is built and should return the content that will be displayed. In most cases, you'll be using a templating library to create HTML content.

Finally, make sure to [register the page](#registering-routes) in the `coronate` function for it to be built.

### Ergonomic returns

The `Page` trait accepts a generic parameter in third position for the return type of the `render` method. This type must implement `Into<RenderResult>`, enabling more ergonomic returns in certain cases.

```rs
impl Page<(), (), String> for HelloWorld {
  fn render(&self, ctx: &mut RouteContext) -> String {
    "Hello, world!".to_string()
  }
}
```

Maudit implements `Into<RenderResult>` for the following types:

- `String`, `Vec<u8>`, `&str`, `&[u8]`
- [Various templating libraries](/docs/templating/)

### Dynamic Routes

Maudit supports creating dynamic routes with parameters. Allowing one to create many pages that share the same structure and logic, but with different content. For example, a blog where each post has a unique URL, e.g., `/posts/my-blog-post`.

To create a dynamic route, export a struct using the `route!` attribute and add parameters by enclosing them in square brackets (ex: `/posts/[slug]`) in the route's path.

In addition to the `render` method, dynamic routes must implement a `routes` method for Page. The `routes` method returns a list of all the possible values for each parameter in the route's path, so that Maudit can generate all the necessary pages.

```rs
use maudit::page::prelude::*;

#[route("/posts/[slug]")]
pub struct Post;

#[derive(Params, Clone)]
pub struct Params {
  pub slug: String,
}

impl Page<Params> for Post {
  fn render(&self, ctx: &mut RouteContext) -> RenderResult {
    let params = ctx.params::<Params>();
    RenderResult::Text(format!("Hello, {}!", params.slug))
  }

  fn routes(&self, ctx: &DynamicRouteContext) -> Routes<Params> {
    vec![Route::from_params(Params {
      slug: "hello-world".to_string(),
    })]
  }
}
```

The route parameters are automatically extracted from the URL and made available through the `ctx.params::<T>()` method in the `RouteContext` struct, providing type-safe access to the values.

```rs
use maudit::page::prelude::*;

#[route("/posts/[slug]")]
pub struct Post;

#[derive(Params, Clone)]
pub struct Params {
  pub slug: String,
}

impl Page for Post {
  fn render(&self, ctx: &mut RouteContext) -> String {
    let slug = ctx.params::<Params>().slug;
    format!("Hello, {}!", slug)
  }

  fn routes(&self, ctx: &DynamicRouteContext) -> Routes<Params> {
    vec![Route::from_params(Params {
      slug: "hello-world".to_string(),
    })]
  }
}
```

The struct used for the parameters must implement `Into<RouteParams>`, which can be done automatically by deriving the `Params` trait. The fields of the struct must implement the `Display` trait, as they will be converted to strings to be used in the final URLs and file paths.

Like static routes, dynamic routes must be [registered](#registering-routes) in the `coronate` function in order for them to be built.

### Endpoints

Maudit supports returning other types of content besides HTML, such as JSON, plain text or binary data. To do this, simply add a file extension to the route path and return the content in the `render` method.

```rs
use maudit::page::prelude::*;

#[route("/api.json")]
pub struct HelloWorldJson;

impl Page for HelloWorldJson {
  fn render(&self, ctx: &mut RouteContext) -> RenderResult {
    RenderResult::Text(r#"{"message": "Hello, world!"}"#.to_string())
  }
}
```

Dynamic routes can also return different types of content. For example, to return a JSON response with the post's content, you could write:

```rs
use maudit::page::prelude::*;

#[route("/api/[slug].json")]
pub struct PostJson;

#[derive(Params, Clone)]
pub struct Params {
  pub slug: String,
}

impl Page<Params> for PostJson {
  fn routes(&self, ctx: &DynamicRouteContext) -> Routes<Params> {
    vec![Route::from_params(Params {
      slug: "hello-world".to_string()
    })]
  }

  fn render(&self, ctx: &mut RouteContext) -> RenderResult {
    let params = ctx.params::<Params>();

    RenderResult::Text(format!(r#"{{"message": "Hello, {}!"}}"#, params.slug))
  }
}
```

Endpoints must also be [registered](#registering-routes) in the `coronate` function in order for them to be built.

### Registering Routes

All kinds of routes must be passed to the `coronate` function in [the entrypoint](/docs/entrypoint) in order to be built.

The first argument to the `coronate` function is a `Vec` of all the routes that should be built. This list can be created using the `routes!` macro to make it more concise.

```rs
use pages::Index;
use maudit::{coronate, routes, BuildOptions, BuildOutput};

fn main() -> Result<BuildOutput, Box<dyn std::error::Error>> {
    coronate(
      routes![Index],
      vec![].into(),
      BuildOptions::default()
    )
}
```
