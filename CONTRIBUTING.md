# Contributing to Maudit

First, a huge **thank you** for dedicating your time to helping us improve Maudit ❤️

> [!Tip]
>
> **New to open source?** Check out [https://github.com/firstcontributions/first-contributions](https://github.com/firstcontributions/first-contributions) for helpful information on contributing

## Before Opening Issues

- **Do not report security vulnerabilities publicly** (e.g., in issues or discussions), please refer to our [security policy](https://github.com/bruits/maudit/blob/main/SECURITY.md).
- **Do not create issues for questions about using Maudit.** Instead, ask your question in our [GitHub Discussions](https://github.com/bruits/maudit/discussions/categories/q-a) or on [our Discord](https://maudit.org/chat).
- **Do not create issues for ideas or suggestions.** Instead, share your thoughts in our [GitHub Discussions](https://github.com/bruits/maudit/discussions/categories/ideas) or on [our Discord](https://maudit.org/chat).
- **Check for duplicates.** Look through existing issues and discussions to see if your topic has already been addressed.
- Please include a reproducible example to help us understand your issue.
- In general, provide as much detail as possible. No worries if it's not perfect, we'll figure it out together.

## Before submitting Pull Requests (PRs)

- **Check for duplicates.** Look through existing PRs to see if your changes have already been submitted.
- PRs don't need to be perfect. Submit your best effort, and we will gladly assist in polishing the work.

## Code of Conduct

We’re committed to fostering a welcoming and respectful community. Any issue, PR, or discussion that violates our [code of conduct](https://github.com/bruits/maudit/blob/main/CODE_OF_CONDUCT.md) will be deleted, and the authors will be **banned**.

## Getting started

Maudit is a fairly standard Rust project with a typical directory structure. It does not rely on any third-party build systems, complex configurations or dependencies in other languages.

### Prerequisites

- Latest stable version of [Rust](https://www.rust-lang.org/)
- (Optional, for the CLI) [Node.js](https://nodejs.org/) and [pnpm](https://pnpm.io/).
  - We recommend installing pnpm using [Corepack](https://pnpm.io/installation#using-corepack).

### Project structure

Maudit is a Rust monorepo using [Cargo workspaces](https://doc.rust-lang.org/book/ch14-03-cargo-workspaces.html).

```bash
├── crates/
│   ├── maudit/ # Maudit
│   ├── maudit-macros/ # Proc macros the library exposes (e.g. #[route])
│   └── maudit-cli/ # Maudit's CLI and dev server
├── benchmarks/ # Benchmarking code
└── examples/ # Various examples showcasing Maudit's capabilities, also used as templates
```

---

Thank you once again for contributing, we deeply appreciate all contributions, no matter how small or big.
