---
cargo/maudit: patch
cargo/maudit-cli: patch
---

The Maudit CLI will now directly rerun the website's binary instead of using Cargo when changes do not require recompilation, this on average speeds up the feedback loop by 300-1000ms.
