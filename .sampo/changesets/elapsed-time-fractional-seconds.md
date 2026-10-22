---
cargo/maudit: patch
cargo/maudit-cli: patch
---

Fixed build and dev server timings being rounded down to whole seconds, e.g. a 1.9s build showing as "1s".
