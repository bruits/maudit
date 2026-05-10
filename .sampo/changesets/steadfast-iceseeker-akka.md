---
cargo/maudit: patch
---

Enabled Rolldown's `resolve_new_url_to_asset` experimental feature to transform `new URL('./path', import.meta.url)` patterns to bundled asset URLs.
