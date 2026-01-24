# Maudit <img align="right" valign="center" width="89" height="75"  src="https://raw.githubusercontent.com/bruits/maudit/main/.github/assets/logo_light.svg#gh-dark-mode-only" alt="Logo of Maudit, a crudely crown" /> <img align="right" valign="center" width="89" height="75"  src="https://raw.githubusercontent.com/bruits/maudit/main/.github/assets/logo.svg#gh-light-mode-only" alt="Logo of Maudit, a crudely crown" />

> A dire coronation, a situation where nobility, power, or status becomes inextricably tied to disastrous circumstances.

[![Website](https://img.shields.io/website?url=https%3A%2F%2Fmaudit.org&up_message=maudit.org&down_message=maudit.org&label=www)](https://maudit.org)
[![Crates.io License](https://img.shields.io/crates/l/maudit)](https://github.com/bruits/maudit/blob/main/LICENSE)
[![Build Status](https://github.com/bruits/maudit/workflows/CI/badge.svg)](https://github.com/bruits/maudit/actions)
[![Current Crates.io Version](https://img.shields.io/crates/v/maudit.svg)](https://crates.io/crates/maudit)
[![CodSpeed Badge](https://img.shields.io/endpoint?url=https://codspeed.io/badge.json)](https://codspeed.io/bruits/maudit)
[![Discord](https://img.shields.io/discord/1323452220817014876)](https://maudit.org/chat/)

[Maudit](https://maudit.org) is a library for generating static websites with Rust.

## Install

The quickest way to get started is with the Maudit CLI.

First, you will need to install Rust 1.89 or later. We recommend using [rustup](https://rustup.rs/).

Then, install the Maudit CLI tool with:

```bash
cargo install maudit-cli
```

You can now create a new project by running the following command:

```bash
maudit init
```

Maudit will ask you a series of questions and generate the project. Once it's done, navigate to the new directory and start the development server:

```bash
cd my-website
maudit dev
```

The first build downloads and compiles dependencies, which will take some time. Subsequent builds will be much faster.

If you wish to use Maudit without the CLI tool, or to integrate Maudit into an existing project, please refer to the [manual installation guide](/docs/manual-install).

## Build

Build your project to the `dist` directory with optimizations suited for production:

```bash
maudit build
```

Preview your build on a local server before deploying:

```bash
maudit preview
```

## Quick links

🌍 Visit our [website](https://maudit.org) to read the [documentation](https://maudit.org/docs) and the latest [news](https://maudit.org/news)

📦 See the [crate](https://crates.io/crates/maudit) on Crates.io, and the [reference](https://docs.rs/maudit/latest/maudit/) on Docs.rs.

🐛 [Report a bug](https://github.com/bruits/maudit/issues). Please read our [contributing guidelines](https://github.com/bruits/maudit/blob/main/CONTRIBUTING.md) and [code of conduct](https://github.com/bruits/maudit/blob/main/CODE_OF_CONDUCT.md) first.

💬 Join the discussion on [Github](https://github.com/bruits/maudit/discussions) and [Discord](https://maudit.org/chat/), if you have any questions, ideas, or suggestions.

## Contributing

Contributions are welcome! See our [Contributor Guide](https://github.com/bruits/maudit/blob/main/CONTRIBUTING.md) for help getting started.

# License

Maudit is licensed under the [MIT License](https://opensource.org/license/MIT). See [LICENSE](LICENSE) for details.
