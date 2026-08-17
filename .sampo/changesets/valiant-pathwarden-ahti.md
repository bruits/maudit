---
cargo/maudit-cli: patch
---

Fixed the development server serving stale static files after they change, the browser now revalidates them on every request.
