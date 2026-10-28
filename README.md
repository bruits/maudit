# Maudit <img align="right" valign="center" width="89" height="75"  src="https://raw.githubusercontent.com/bruits/maudit/main/.github/assets/logo_light.svg#gh-dark-mode-only" alt="Logo of Maudit, a crudely crown" /> <img align="right" valign="center" width="89" height="75"  src="https://raw.githubusercontent.com/bruits/maudit/main/.github/assets/logo.svg#gh-light-mode-only" alt="Logo of Maudit, a crudely crown" />

> A dire coronation, a situation where nobility, power, or status becomes inextricably tied to disastrous circumstances.

A library for generating static websites with Rust.

* **Built by you**: Maudit is not an opaque SSG that you feed configuration into and hope for the best. It is based entirely on code you yourself write.
* **Lightning fast rebuilds**: Maudit performs best when you have content that changes multiple times while the layout is constant.
* **Does just what you need**: Maudit is *only* made to render static sites, so if you need a static site this tool is just for you.

Read the [documentation](https://maudit.org/docs/) on [maudit.org](https://maudit.org), or join us on [Discord](https://maudit.org/chat/).

```rust
use maudit::route::prelude::*;

#[route("/hello-world")]
pub struct HelloWorld;

impl Route for HelloWorld {
  fn render(&self, ctx: &mut PageContext) -> impl Into<RenderResult> {
    "Hello, world!"
  }
}
```

```rust
use routes::HelloWorld;
use maudit::{coronate, routes, content_sources, BuildOptions, BuildOutput};

fn main() -> Result<BuildOutput, Box<dyn std::error::Error>> {
    coronate(
      routes![HelloWorld],
      content_sources![],
      BuildOptions::default()
    )
}
```

## Usage

Getting started is easy!

**Step 1**: install maudit

```bash
cargo install maudit-cli
```

**Step 2**: create your project

```bash
maudit init
```

See [the docs](https://maudit.org/docs/quick-start/) for more info about where to go from here.

## Crates

Maudit is a monorepo that contains the following crates (Rust packages):

| Name            | Description                                                | Registry                                                                                                                                       | README                                     |
| --------------- | ---------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------ |
| `maudit`        | Library for generating static websites                     | <a href="https://crates.io/crates/maudit"><img alt="Maudit Crates.io Version" src="https://img.shields.io/crates/v/maudit"></a>                | [README](https://github.com/bruits/maudit/blob/main/crates/maudit/README.md)        |
| `maudit-cli`    | CLI and dev server to operate on Maudit projects           | <a href="https://crates.io/crates/maudit-cli"><img alt="Maudit CLI Crates.io Version" src="https://img.shields.io/crates/v/maudit-cli"></a>    | [README](https://github.com/bruits/maudit/blob/main/crates/maudit-cli/README.md)    |
| `maudit-macros` | Procedural macros exposed by the library (e.g. `#[route]`) | <a href="https://crates.io/crates/maudit-macros"><img alt="Maudit Macros Crates.io Version" src="https://img.shields.io/crates/v/maudit-macros"></a> | [README](https://github.com/bruits/maudit/blob/main/crates/maudit-macros/README.md) |
| `oubli`         | Library for generating documentation websites with Maudit  | <a href="https://crates.io/crates/oubli"><img alt="Oubli Crates.io Version" src="https://img.shields.io/crates/v/oubli"></a>                   | [README](https://github.com/bruits/maudit/blob/main/crates/oubli/README.md)         |

## Acknowledgements

Maudit is an open-source project born from [Bruits](https://bruits.org/), a Rust-focused collective 💛
