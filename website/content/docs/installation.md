---
title: "Installation"
description: "How to install Maudit"
section: "getting-started"
---

## Prerequisites

- [Rust (1.83 or later)](https://www.rust-lang.org)
- A code editor (e.g. Visual Studio Code, RustRover, Helix, etc.)
- A terminal emulator (e.g. Windows Terminal, Terminal.app, Ghostty, etc.)

We recommend using [rustup](https://rustup.rs/) to install Rust. Maudit is intended to be used with the latest stable version of Rust and does not require nightly features.

Once Rust is installed, run the following command to ensure the latest stable version of Rust is being used:

```bash
rustup default stable
```

## Installing Maudit

Maudit provides a CLI tool for interacting with websites created using the library and generating new ones. To install the CLI tool, run the following command:

```bash
cargo install maudit-cli
```

This will install the `maudit` binary in your Cargo bin directory. You can now run `maudit --help` to see the available commands and options.

If you do not wish to use the CLI, or are integrating Maudit into an existing project, follow the instructions in the [manual installation guide](/docs/manual-install).

## Creating a new project

To create a new Maudit project, run the following command:

```bash
maudit init
```

Maudit will then ask you a series of questions to configure your project. Once complete, you can navigate to the project directory and start the development server using `maudit dev`:

```bash
cd my-website
maudit dev
```
