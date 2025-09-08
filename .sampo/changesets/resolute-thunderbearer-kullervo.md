---
packages:
  - maudit
release: minor
---

Adds support for dynamic routes with properties. In addition to its parameters, a dynamic route can now provide additional properties that can be used during rendering.

```rs
use maudit::page::prelude::*;

#[route("/posts/[slug]")]
pub struct Post;

#[derive(Params, Clone)]
pub struct Params {
  pub slug: String,
}

#[derive(Clone)]
pub struct Props {
  pub title: String,
  pub content: String,
}

impl Page<Params, Props> for Post {
  fn render(&self, ctx: &mut RouteContext) -> RenderResult {
    let params = ctx.params::<Params>();
    let props = ctx.props::<Props>();

    format!(
      "<h1>{}</h1><p>{}</p><small>Slug: {}</small>",
      props.title, props.content, params.slug
    ).into()
  }

  fn routes(&self, ctx: &mut DynamicRouteContext) -> Routes<Params, Props> {
    vec![Route::from_params_and_props(
      Params {
        slug: "hello-world".to_string(),
      },
      Props {
        title: "Hello World".to_string(),
        content: "This is my first post.".to_string(),
      },
    )]
  }
}
```

For more information on dynamic routes, see the [Routing documentation](https://maudit.org/docs/routing/#dynamic-routes).
