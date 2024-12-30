---
title: "Routing"
description: "How to create pages and routes in Maudit"
section: "core-concepts"
---

### Static Routes

Maudit uses a simple and intuitive API to define routes and pages. To create a new page, define a struct that implements the `Page` trait, adding the `#[route]` attribute to the struct definition with the path of the route as an argument.

```rust
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

### Dynamic Routes

Maudit supports creating dynamic routes with parameters. Allowing one to create many pages that share the same structure and logic, but with different content.

For example, one could create a route that matches `/posts/[slug]` and renders a page with the content of the post with the given slug.

To create a dynamic route, export a struct using the `route!` macro and add parameters to the route path using the `[]` syntax. For example, to create a route that matches `/posts/[slug]`, you would write:

```rust
use maudit::route::prelude::*;

#[route("/posts/[slug]")]
pub struct Post;

impl Page for Post {
  fn render(&self, ctx: &mut RouteContext) -> String {
    format!("Hello, {}!", ctx.params.get("slug").unwrap())
  }
}
```

In addition to the `Page` trait, dynamic routes must implement the `DynamicRoute` trait for their struct. This trait requires a `routes` function that returns a list of all the possible values for each parameter in the route's path.

```rust
use maudit::{page::prelude::*, FxHashMap};

#[route("/posts/[slug]")]
pub struct Post;

impl DynamicRoute for Post {
  fn routes(&self, ctx: &DynamicRouteContext) -> Vec<RouteParams> {
    let mut routes = FxHashMap::default();
    routes.insert("slug".to_string(), "hello-world".to_string());

    vec![RouteParams(routes)]
  }
}

impl Page for Post {
  fn render(&self, ctx: &mut RouteContext) -> RenderResult {
    RenderResult::Text(format!("Hello, {}!", ctx.params.get("slug").unwrap()))
  }
}
```

The `RouteParams` type is a [newtype](https://doc.rust-lang.org/rust-by-example/generics/new_types.html) around a `FxHashMap<String, String>`, representing the raw parameters as if they were directly extracted from an URL.

Interacting with HashMaps in Rust can be a bit cumbersome, so Maudit provides the ability to use a custom struct to define your params and easily convert them into `RouteParams` after.

```rust
#[derive(Params)]
pub struct Params {
  pub slug: String,
}

impl DynamicRoute for Post {
  fn routes(&self, ctx: &DynamicRouteContext) -> Vec<RouteParams> {
    let routes = vec![ArticleParams {
      slug: "hello-world".to_string(),
    }];

    RouteParams::from_vec(routes)
  }
}
```

This struct can also be used when defining the `Page` implementation, making it possible to access the parameters in a type-safe way. For more information on how to use the `Params` derive, see the [TODO](TODO) section.

```rust
#[derive(Params)]
pub struct Params {
  pub slug: String,
}

impl Page for Post {
  fn render(&self, ctx: &mut RouteContext) -> RenderResult {
    let params = ctx.params::<Params>();

    RenderResult::Text(format!("Hello, {}!", params.slug))
  }
}
```

Like static routes, dynamic routes must be [registered](#registering-routes) in the `coronate` function in order for them to be built.

### Endpoints

Maudit supports returning other types of content besides HTML, such as JSON or plain text. To do this, simply add a file extension to the route path and return the content in the `render` method.

```rust
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

```rust
use maudit::page::prelude::*;

#[route("/api/[slug].json")]
pub struct PostJson;

#[derive(Params)]
pub struct Params {
  pub slug: String,
}

impl DynamicRoute for PostJson {
  fn routes(&self, ctx: &DynamicRouteContext) -> Vec<RouteParams> {
    let routes = vec![Params { slug: "hello-world".to_string() }];

    RouteParams::from_vec(routes)
  }
}

impl Page for PostJson {
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

```rust
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
