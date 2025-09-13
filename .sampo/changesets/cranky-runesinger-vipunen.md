---
packages:
  - maudit
release: minor
---

The data URI and average RGBA for thumbnails is now calculated lazily, as such the `average_rgba` and `data_uri` fields have been replaced by methods.
