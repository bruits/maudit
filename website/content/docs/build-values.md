---
title: "Build values"
description: "Compute a value once per build and share it across every page that needs it."
section: "advanced"
---

Sometimes many pages need the same value: a bit of configuration, an expensive lookup table, a cache-busting hash, an index built from your content. Recomputing it in every page is wasteful, and computing it once in a global breaks in ways that only show up on incremental builds.

A build value is computed once per build and shared across every page that reads it.

## Defining a build value

Declare a build value as a `static` with [`BuildValue::new`](https://docs.rs/maudit/latest/maudit/build_value/struct.BuildValue.html). The closure computes the value and returns it:

```rs
use maudit::route::prelude::*;

static SITE_VERSION: BuildValue<String> = BuildValue::new(|_ctx| {
    std::env::var("SITE_VERSION").unwrap_or_else(|_| "dev".into())
});
```

The closure can't capture anything from its surroundings, which is what lets it live in a `static`. It receives a context for reading content and other build values, but the value itself can be anything: a config string, a parsed file, a lookup table.

## Reading a build value

Read a build value from a route's `render` with [`PageContext#build_value`](https://docs.rs/maudit/latest/maudit/route/struct.PageContext.html). It returns an `Rc<T>`:

```rs
#[route("/")]
pub struct Index;

impl Route for Index {
    fn render(&self, ctx: &mut PageContext) -> impl Into<RenderResult> {
        let version = ctx.build_value(&SITE_VERSION); // Rc<String>

        format!("<footer>Built from {version}</footer>")
    }
}
```

The closure runs at most once per build, the first time any page reads the value. Every page that reads it gets the same result.

## Scoped to one build

A build value is scoped to a single build. In a long-running process like `maudit dev`, each rebuild recomputes it. A plain `static` computed with `LazyLock` or `OnceLock` behaves differently: it runs once for the life of the process and never updates, so a value that should change between builds goes stale in the dev server. A build value stays correct across rebuilds.

## Deriving from content

If the closure reads content, Maudit tracks those reads. Every page that reads the value depends on the content the value was derived from, so incremental builds re-render exactly the right pages.

The context exposes the same content API you use in pages: `entries()`, `get_entry(id)`, `get_entry_safe(id)`, and `entry.data(ctx)`.

```rs
#[markdown_entry]
pub struct Article {
    pub title: String,
}

static ARTICLE_TITLES: BuildValue<Vec<String>> = BuildValue::new(|ctx| {
    let articles = ctx.content::<Article>("articles");

    let mut titles: Vec<String> = Vec::new();
    for entry in articles.entries() {
        titles.push(entry.data(ctx).title.clone());
    }
    titles.sort();
    titles
});
```

Every page that reads `ARTICLE_TITLES` re-renders when an article changes and stays cached otherwise. You don't manage that caching yourself.

A value derived from something Maudit doesn't track, like an environment variable or an external file, is still computed once per build, but its readers won't re-render if only that input changes between incremental builds. A full rebuild picks up the new value.

## Globals break on incremental builds

A common instinct is to compute the value in one route's `render` and stash it in a global for other routes to read. This works on a full build but breaks on incremental ones.

On an incremental build, the route that computed the value can be served from cache, so Maudit skips its `render` and the global is never set. The other pages then read an empty value, or panic. It fails only on local rebuilds, and only depending on which file you last touched.

Build values avoid this by being computed on read rather than as a side effect of a route's `render`. It doesn't matter which pages are cache hits; the value is computed the first time it's needed.

## What a build value can and can't do

Inside the closure you can read content sources and other build values (they nest, and their dependencies fold in). You can't:

- Register assets like images, scripts, or styles. Those belong to a route's `render`.
- Render a content entry's body with `entry.render(ctx)`. Rendering needs a full `PageContext`. If you need a value derived from rendered output, derive it from the inputs instead: for a cache-busting hash, hash the source data rather than the rendered HTML, which changes at the same times without the cost of rendering.

Build values are read during `render`, not during `pages`.

## When to reach for one

Use a build value when the same value is read by several pages and isn't trivial to recompute per page: a shared config, an expensive computation, a content-derived index, a site-wide hash.

For a cheap, one-page computation, compute it inline in that page instead. A build value earns its place when you want to compute something once and share it.
