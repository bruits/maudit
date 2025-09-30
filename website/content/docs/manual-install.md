---
title: "Manual installation"
description: "While it is recommended to use the CLI tool to create and manage Maudit projects, it is also possible to manually install Maudit like any other Rust library."
---

Create a new Rust project using Cargo, specifying the `--bin` flag to create a binary project:

```bash
cargo new my-website --bin
cd my-website
```

Next, add Maudit as a dependency in your `Cargo.toml` file, or run `cargo add maudit` to do so automatically. You might also want to add [Maud](https://maud.lambda.xyz/) as a dependency if you plan to use it for templating.

```toml
[dependencies]
maudit = "0.6"
maud = "0.27" # optional
```

Voilà! You can now use Maudit in your project. Check out the rest of the [documentation](/docs) for more information on how to use Maudit, or if you prefer jumping straght into the code, take a look at the [examples](https://github.com/bruits/maudit/tree/main/examples) and the [API documentation](https://docs.rs/maudit).
