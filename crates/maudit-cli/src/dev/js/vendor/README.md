# Vendored libraries for the Maudit CLI dev server.

Since the JS part of the Maudit CLI is built at build time through a Rust build script, we cannot rely on a package manager to fetch dependencies, as people downloading the Maudit CLI from crates.io won't have access to npm, pnpm, yarn, etc.

As such, we vendor the dependencies we need here. They are copied as-is from their respective repositories and include their license information at the top of the file.

An alternative to this approach would be to pre-bundle the JS code and ship the bundled version with the crate, but it's a lot more cumbersome in development so while things are still evolving, we prefer this approach.
