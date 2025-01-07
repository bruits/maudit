---
title: "Incremental builds"
description: "Welcome to the Maudit documentation!"
section: "advanced"
---

Maudit has partial support for incremental builds. When enabled, Maudit will only rebuild pages whose content or assets they depend on have changed.

## Limitations

Due to Maudit's architecture, which includes static parts requiring compilation (e.g., \*.rs files), full support for incremental builds is not feasible. Incremental builds are most effective for content sources relying on local files and local assets, such as images and stylesheets.

As a result, incremental builds in Maudit currently have the following limitations:

- Changes to **any** part of the project that requires compilation will require a full rebuild. For example, changing a `<h1>` to a `<h2>` in a Maud template will necessitate a recompilation of the project and a full rebuild.
- Maudit can only track changes to dependencies that are added through Maudit APIs. For example, loading a file from disk manually through `std::fs` or fetching data from the internet will not trigger a rebuild if the file or data changes. Additionally, calls must be exhaustive for all potentially changing content. For example, if a page decides on an image to use based on an environment variable, both images should be added to the page's assets for the page to be rebuilt when either image changes.
- Getting the information needed to determine which pages should be rebuilt can often be slower than just rebuilding the entire project, especially in small-to-medium sized projects.

Certain of these limitations may be alleviated in the future, but ultimately some of these limitations are inherent to the nature of Maudit and the Rust programming language and may not ever be fully resolved.

There are also some current limitations to Maudit's incremental builds that may be improved in the future:

- Change detection to content sources is not fine-grained. For example, changing a single article in a blog will trigger a rebuild of all pages that depend on the blog's content source.
- Imported assets are reprocessed on every build, even if they have not changed.

Given all these limitations, and the fact that Maudit is often fast enough without incremental builds, making incremental builds perfect is not a priority for the project. The complexity and constraints added by incremental builds generally makes it more beneficial for the project to focus on improving overall compile and build times instead.

## Enabling incremental builds

To enable incremental builds in Maudit, set the `incremental` field in the `BuildOptions` struct to `true`.

To properly handle recompilation, Maudit also requires a unique identifier to be added to each build of the project. If the project has been created with `maudit new`, this identifier is already exported as the build-time `MAUDIT_BUILD_ID` environment variable from the `build.rs` script and can be accessed with the `buildId!()` macro.

```rust
use maudit::{coronate, buildId, routes, BuildOptions, BuildOutput};

fn main() -> Result<BuildOutput, Box<dyn std::error::Error>> {
  coronate(
    // ...
    BuildOptions {
      incremental: true,
      build_id: buildId!(),
      ..Default::default()
    }
  )
}
```

If your project has not been created with `maudit new`, you can generate a build ID with the `maudit_build_script` crate. Add the `maudit_build_script` crate to your project's build dependencies and call the `generate_build_id` function in your `build.rs` script.

```toml
[build-dependencies]
maudit_build_script = "0.1"
```

```rust
use maudit_build_script::generate_build_id;

fn main() {
    generate_build_id();
}
```

Note that incremental builds require an initial full build with the option enabled before taking effect. This is because Maudit needs to generate a cache of the project's content and assets to compare against in subsequent builds.

**Caution**: Using this feature makes builds of your website non-deterministic.
