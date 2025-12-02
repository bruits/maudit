---
cargo/maudit: patch
---

Assets-related methods now all return Result, returning errors whenever files cannot be read or some other IO issue occurs. This makes it slightly more cumbersome to use, of course, however it makes it much easier to handle errors and return better error messages whenever something goes wrong.
