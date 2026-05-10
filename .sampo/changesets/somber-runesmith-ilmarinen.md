---
cargo/maudit-cli: minor
---

Added support for `_headers` files to the dev server. Drop a `_headers` file in your static directory of your website and the dev server will apply its rules to responses, matching the behaviour of hosts like Cloudflare and Netlify that supports those files in production.
