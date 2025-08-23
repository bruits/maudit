---
title: "Quick Start"
section: "getting-started"
---

In this guide, you'll learn how to create a Maudit website and the general basis of Maudit in a few minutes of reading.

If you prefer to read more detailed explanations, including exploration of various Maudit concepts, please read the [the tutorial]().

**This guide assumes that you have Rust installed and are familiar with the terminal.**

## Installation

First install the Maudit CLI.

```shell
cargo install maudit-cli
```

## Generating a Maudit project

Run the following command to generate a Maudit project.

The command will suggest many templates to begin from. Some of which are usable as-is (such as the Blog example)

```shell
maudit init
```

Once done, `cd` into the directory you have chosen to create your project in.

## Running your project

Use the Maudit CLI to build, run in development mode or preview your project.

The `maudit build` command will build your project to the `dist` directory, ready to be deployed.

`maudit dev` will serve your website on a local server, automatically rebuilding and refreshing the page on changes.

`maudit preview` will serve your website on a local server with various optimization enabled to imitate what a real production server would do, and is intended to preview your built website before deploying.

## Creating pages

Pages in Maudit are created directly in Rust, using `.rs` files.

To create a page, create a `.rs` file with a public struct using the `route` attribute, which take the path of the route as sole parameter.

All of Maudit's useful imports for pages can be imported using the prelude from `maudit::page`.

```rs
use maudit::page::prelude::*;

#[route("/hello-world")]
pub struct HelloWorld;
```

Every page must `impl` the `Page` trait, with the required method `render`.

```rs
impl Page for HelloWorld {
  fn render(&self, ctx: &mut RouteContext) -> RenderResult {
    "Hello, world!".into()
  }
}
```

Finally, pages' struct must be passed to the `coronate` function in the project's `main.rs`

```rs
use pages::HelloWorld;
use maudit::{coronate, routes, content_sources, BuildOptions, BuildOutput};

fn main() -> Result<BuildOutput, Box<dyn std::error::Error>> {
    coronate(
      routes![Index],
      content_sources![],
      BuildOptions::default()
    )
}
```
