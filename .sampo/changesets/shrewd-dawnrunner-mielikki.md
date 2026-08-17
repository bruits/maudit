---
cargo/maudit-cli: patch
---

Fixed the development server ignoring changes in directories named `dist`, `target` or `.git` anywhere in the project, such as `static/dist`. Only the ones at the root are skipped now.
