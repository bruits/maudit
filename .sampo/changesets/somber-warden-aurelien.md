---
cargo/maudit: minor
cargo/maudit-macros: minor
---

Adds support for incremental builds. Subsequent builds will now only re-render pages whose content or assets have changed, making rebuilds significantly faster. This is enabled by default. To disable it, set `incremental: false` in your `BuildOptions`:

```rust
use maudit::{content_sources, coronate, routes, BuildOptions, BuildOutput};

fn main() -> Result<BuildOutput, Box<dyn std::error::Error>> {
  coronate(
    routes![],
    content_sources![],
    BuildOptions {
      incremental: false,
      ..Default::default()
    },
  )
}
```
