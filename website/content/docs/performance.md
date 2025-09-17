---
title: "Performance"
description: "Learn how to improve the build times and performance of your Maudit site."
section: "guide"
---

Maudit can generally build websites pretty quickly, but there are a few strategies you can use to improve build times and the performance of your site.

A Maudit project is a normal Rust project, so [any performance optimizations that apply to Rust projects](https://nnethercote.github.io/perf-book/build-configuration.html#minimizing-compile-times) also apply to Maudit projects, but there are a few specific strategies that can help improve the performance of your Maudit site.

## In development

### Cargo settings

We recommend using the following settings in your `Cargo.toml` to improve build times during development. This will increase the optimization level of your dependencies without making the compile time of your own crate longer.

```toml
[profile.dev.package."*"]
opt-level = 3
```

This is particularly relevant if you are processing a lot of images, as there is a large difference in performance between debug and release builds of the crates Maudit uses for image processing.

### Disabling heavy features during development

When running through `maudit dev`, the `is_dev()` function will return `true`, allowing you to conditionally disable features that are slow to build or run during development.

```rs
use maudit::is_dev;

if !is_dev() {
  // Do something slow
}
```

Running `cargo run` will show which pages of your site are slow to build, allowing you to identify bottlenecks in your build process.

### Preventing build directory block

As Maudit recompile your project on every change, it is possible to run into issues where the build directory is blocked by another process, [most commonly `rust-analyzer` in your editor.](https://github.com/rust-lang/rust-analyzer/issues/4616), slowing down builds significantly.

While this does improve the time it takes to get feedback on changes, note that changing `rust-analyzer` settings to use a different build directory will use a lot of disk space.

## In production

### Release builds

If not using `maudit build`, ensure you are building your project in release mode using `cargo build --release` or `cargo run --release`. This will significantly improve the performance of your site and is a common pitfall for new users.

### Caching

Make sure to cache the `target` directory between builds in your CI/CD pipeline. This will very significantly improve build times, especially if you have a lot of dependencies or processed images. Platforms like Netlify or Vercel will automatically cache the `target` directory for you.
