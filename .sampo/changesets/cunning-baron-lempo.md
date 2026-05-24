---
cargo/maudit: minor
---

Bundle filenames now properly gets updated for every transitive dependency (CSS-referenced fonts and images, JS-imported assets, etc.). This was done by rewriting urls after bundling, which does results in worse performance, but at least ensure the URLs are always correct.
