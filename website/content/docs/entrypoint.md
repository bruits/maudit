---
title: "Entrypoint"
description: "Learn how to set up the entrypoint for your Maudit project."
section: "core-concepts"
---

At the core of a Maudit project is the [`coronate`](https://docs.rs/maudit/latest/maudit/fn.coronate.html) function. This function starts the build process and generates the output files. It is the entrypoint to your project and is where you'll pass the pages, content and options that make up your website.

In a `main.rs` file, import the `coronate` function and call it to build your project. Here is an example of a simple Maudit project:

```rs
use maudit::{coronate, routes, BuildOptions, BuildOutput};
use routes::Index;

fn main() -> Result<BuildOutput, Box<dyn std::error::Error>> {
  coronate(routes![Index], vec![].into(), BuildOptions::default())
}
```

## Registering Routes

All kinds of routes must be passed to the `coronate` function in order for them to be built.

The first argument to the `coronate` function is a `Vec` of all the routes that should be built. For the sake of ergonomics, the `routes!` macro can be used to create this list.

```rs
use routes::Index;

coronate(
  routes![Index],
  vec![].into(),
  BuildOptions::default()
)
```

See the [Routing](/docs/routing) documentation for more information on how to define routes.

## Content

The second argument to the `coronate` function is a list of content sources. Content sources are used to load content and data from various sources, such as the filesystem (ex: a folder of markdown files), a database, or a remote API.

```rs
use maudit::content::content_sources;

fn main() {
  coronate(
    routes![
      // ...
    ],
    content_sources![
      "source_name" => loader(...),
    ],
    Default::default()
  );
}
```

See the [Content](/docs/content) documentation for more information on how to define content sources.

## Options

The third argument to the `coronate` function is a `BuildOptions` struct. This struct contains various options that can be used to customize the build process.

```rs
use maudit::BuildOptions;

coronate(
  routes![
    // ...
  ],
  content_sources![
    // ...
  ],
  BuildOptions {
    output_dir: "public".into(),
    ..Default::default()
  }
);
```

For a full list of options, see the [`BuildOptions`](https://docs.rs/maudit/latest/maudit/struct.BuildOptions.html) reference.
