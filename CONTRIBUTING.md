# Contributing Guidelines

First, a huge **thank you** for dedicating your time to helping us improve Maudit ❤️

> [!Tip]
> **New to open source?** Check out [https://github.com/firstcontributions/first-contributions](https://github.com/firstcontributions/first-contributions) for helpful information on contributing

## Philosophy

**Maudit is about making static websites:** no SSR, no hybrid rendering, our focus is to make the best static website generator we can. **A website's structure changes less often than its content:** compiled Rust is an acceptable cost for the rare layout change, but content iteration must stay cheap. **Maudit is a library, not a framework:** pages and components are plain Rust structs that remain usable outside the build pipeline, so we prefer APIs that compose in isolation. Read more in our [philosophy](https://maudit.org/docs/philosophy/) page.

We're also committed to fostering a welcoming and respectful community. Any issue, PR, or discussion that violates our [code of conduct](./CODE_OF_CONDUCT.md) will be deleted, and the authors will be **banned**.

## Before Opening Issues

- **Do not report security vulnerabilities publicly** (e.g., in issues), please refer to our [security policy](./SECURITY.md).
- **Do not create issues for questions about using Maudit.** Instead, ask on our [Discord](https://maudit.org/chat/).
- **For ideas or feature suggestions**, open a [feature request issue](https://github.com/bruits/maudit/issues/new?template=02-feature-request.yml) or chat about it first on [Discord](https://maudit.org/chat/).
- **Check for duplicates.** Look through existing issues to see if your topic has already been addressed.
- In general, provide as much detail as possible, including a reproducible example when applicable. No worries if it's not perfect, we'll figure it out together.

## Before submitting Pull Requests (PRs)

- **Check for duplicates.** Look through existing PRs to see if your changes have already been submitted.
- **Check Clippy warnings.** Run `cargo clippy --all --all-targets` to ensure your code adheres to Rust's best practices.
- **Run formatting.** Run `cargo fmt --all` to ensure your code is properly formatted.
- **Write and run tests.** If you're adding new functionality or fixing a bug, please include tests to cover it. Run `cargo test --all` to ensure all existing tests pass.
- **Write a changeset.** Run `sampo add` to create a new changeset file describing your changes.
- Prefer small, focused PRs that address a single issue or feature. Larger PRs can be harder to review, and can often be broken down into smaller, more manageable pieces.
- PRs don't need to be perfect. Submit your best effort, and we will gladly assist in polishing the work.

## Quality Guidelines

- Prefer self-documenting code first, with expressive names and straightforward logic. Comments should explain *why* (intent, invariants, trade-offs), not *how*. Variable and function names should be clear and descriptive, not cryptic abbreviations. Avoid hidden state and side effects.
- Tests should assert observable behavior (inputs/outputs, effects), not internal implementation details. Keep tests deterministic and independent of global state.
- For errors, use typed error enums in library crates (derived with `thiserror`). Per-crate `pub type Result<T>` aliases for ergonomic signatures. Add context at the boundary (CLI) rather than deep in core, keep library error messages concise.
- Prefer `?` propagation when possible, and reserve `.expect()`/`.unwrap()` for cases where failure is a programmer bug (e.g. hardcoded regex literals, test helpers).
- Document any new public APIs, configuration options, or user-facing changes in the relevant README files. If you're unsure where or how to document something, just ask and we'll help you out.
- We deeply value idiomatic, easy-to-maintain Rust code. Avoid code duplication when possible. And prefer clarity over cleverness, and small focused functions over dark magic.
- Explicit `use` imports for standard library types (e.g. `use std::collections::HashMap;`).

## Writing Changesets

Maudit uses [Sampo](https://github.com/bruits/sampo) to manage changelogs and versioning. Every user-facing change should ship with a changeset that lands in the changelog of the next release.

**Structure:**
1. **Breaking prefix (if applicable):** `**⚠️ breaking change:**`
2. **Verb:** `Added`, `Removed`, `Fixed`, `Changed`, `Deprecated`, or `Improved`.
3. **Description**.
4. **Usage example (optional):** A minimal snippet if it clarifies the change.

**Description guidelines:** concise (1-2 sentences), specific (mention the command/option/API), actionable (what changed, not why), user-facing (written for changelog readers), and in English. Don't detail internal implementation changes.

## Getting Started

Maudit is a Rust monorepo using [Cargo workspaces](https://doc.rust-lang.org/book/ch14-03-cargo-workspaces.html), and contains multiple crates (Rust packages) in the `crates/` directory. We do not rely on any third-party build systems, complex configurations or dependencies in other languages. The only prerequisite is to have the latest stable version of [Rust](https://www.rust-lang.org/) installed.

For the website, the examples using Tailwind, the CLI's JavaScript pieces, and the end-to-end tests, you'll also need [Node.js](https://nodejs.org/) and [pnpm](https://pnpm.io/). We recommend installing pnpm via [Corepack](https://pnpm.io/installation#using-corepack).

### Maudit

`maudit` is the core library — a static site generator that lets you describe routes, pages, and content collections in plain Rust. It leans on [`maud`](https://docs.rs/maud/latest/maud/) for type-safe HTML templating, and bundles its own asset pipeline ([`rolldown`](https://rolldown.rs/) for JS/CSS, Markdown rendering via [`pulldown-cmark`](https://docs.rs/pulldown-cmark/latest/pulldown_cmark/), syntax highlighting via [`syntect`](https://docs.rs/syntect/latest/syntect/), and image processing). Most changes here are user-facing, so update the website docs whenever you touch the public surface.

### Maudit CLI

`maudit-cli` is the `maudit` binary: project scaffolding, the development server, and the file-watcher live here. It wires [`clap`](https://docs.rs/clap/latest/clap/) for command parsing, [`tokio`](https://docs.rs/tokio/latest/tokio/) + [`axum`](https://docs.rs/axum/latest/axum/) for the dev server (with a WebSocket-driven live reload), and [`notify`](https://docs.rs/notify/latest/notify/) for filesystem watching. The CLI also ships a small JavaScript layer under `js/`, so changes that affect the dev-server frontend may require `pnpm` as well. Run commands locally with `cargo run -p maudit-cli -- <command>`.

### Maudit Macros

`maudit-macros` is a `proc-macro` crate that exposes the attribute and derive macros consumed by `maudit` (notably `#[route]`). It depends on [`syn`](https://docs.rs/syn/latest/syn/) and [`quote`](https://docs.rs/quote/latest/quote/). When you change a macro, exercise it from a real consumer (an example or the website) rather than only via unit tests — proc-macro errors are easier to read at the call site.

### Oubli

`oubli` is a sibling library that builds documentation websites *with* Maudit. It depends on `maudit` and `maud`, and will powers most of Bruits ecosystem's documentation websites. Treat it as a downstream user of `maudit`: breaking changes in `maudit`'s public API ripple here first.

### Other workspaces

Beside the Cargo crates, the repository also contains an `examples/` directory (small projects used as starter templates and as fixtures for the CLI), a `benchmarks/` directory wired to [CodSpeed](https://codspeed.io/bruits/maudit), an `e2e/` directory with end-to-end tests run via `pnpm test:e2e`, and a `website/` workspace that builds [maudit.org](https://maudit.org).

---

Thank you once again for contributing, we deeply appreciate all contributions, no matter how small or big.
