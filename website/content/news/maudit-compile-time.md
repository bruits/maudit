---
title: "Maudit is faster than most other static website generators, but"
description: "Your website changes less often than its content"
author: The Maudit Team
date: 2025-10-02
---

Objectively speaking, unless it does something weird, a binary using Maudit will generate a website pretty fast. This is expected: Maudit is pretty fast, Rust is pretty fast, native binaries are pretty fast, it checks out.

## _However,_

If a Maudit project is a Rust project, and [Rust projects are slow to compile](https://www.reddit.com/r/rust/comments/xna9mb/why_are_rust_programs_slow_to_compile/) and you need to compile to build your website, doesn't that make Maudit slow by default?

Yes, **but**, there are a few things to consider:

- The slow completely cold compile and download are rare (much like you don't run `npm install` before every build)
- Incremental warm builds are not that slow (< 3s~), and do not necessarily get slower as your website gets larger. The same blog with 5000 and 5 articles compile in the same amount of time (unless they're 5000 different pages and layouts, in which case, well)

And most importantly: **Not every change require recompilation.** Updating your Markdown content, updating frontend JavaScript or CSS, updating some images assets all don't require recompilation and are most definitely more common changes than changing your project's logic.

## Workarounds

If your layouts do change often and you don't want to recompile your project on every change, using a runtime templating language like [minijinja](https://github.com/mitsuhiko/minijinja) or [Tera](https://keats.github.io/tera/docs/) is a great solution. You can load your templates at runtime, and thus change them without recompiling.

### Even further beyond

You can push it even further! [Routes paths are not static, they can be fully dynamic](https://maudit.org/docs/routing/#:~:text=The%20path%20can%20be%20any%20Rust%20expression) as such, you could load your routes fully at runtime.

> This example is available in the [examples/runtime-to-the-max](https://github.com/bruits/maudit/tree/main/examples/runtime-to-the-max) directory of the Maudit repository.

```rust
use maudit::{BuildOptions, BuildOutput, content_sources, coronate, route::prelude::*};

#[route(format!("/dynamic/{}/", self.dynamic_page.0))]
struct Dynamic {
  dynamic_page: (String, String),
}

impl Route for Dynamic {
  fn render(&self, _: &mut PageContext) -> impl Into<RenderResult> {
    self.dynamic_page.1.clone()
  }
}

fn main() -> Result<BuildOutput, Box<dyn std::error::Error>> {
  let routes: Vec<Box<dyn FullRoute>> = std::fs::read_to_string("pages.txt")?
    .lines()
    .filter_map(|line| {
      let mut parts = line.splitn(2, ": ");
      match (parts.next(), parts.next()) {
        (Some(name), Some(content)) => Some((name.to_string(), content.to_string())),
        _ => None,
      }
    })
    .map(|dynamic_page| Box::new(Dynamic { dynamic_page }) as Box<dyn FullRoute>)
    .collect();

  coronate(
    &routes.iter().map(|r| r.as_ref()).collect::<Vec<_>>(),
    content_sources![],
    BuildOptions::default(),
  )
}
```

where `pages.txt` looks like this:

```txt
hello: Hello!
another_page: Another Page
imnested/index: I'm nested!
```

This is my CMS at home, don't judge me. More seriously, you could imagine loading pages from an actual CMS, a database, a `.json` file etc. And thus, combined with runtime templating, be able to add an infinite amount of pages without ever recompiling.

The posibilities are infinite here, perhaps your website will never need recompilation again, who knows.
