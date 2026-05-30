---
cargo/maudit-cli: patch
---

Fixes projects created with `maudit init` failing to compile out of the box. Previously the generated `Cargo.toml` could pin a version of `maudit` that didn't match the downloaded template, so a fresh project would fail to build until you bumped the version by hand.
