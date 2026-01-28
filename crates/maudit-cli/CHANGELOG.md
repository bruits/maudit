# maudit-cli

## 0.7.0 — 2026-01-28

### Minor changes

- [a5b49ad](https://github.com/bruits/maudit/commit/a5b49adacbbbc18506e3157ade0547b60e35348a) Adds support for passing a port to the dev using the `--port` option. — Thanks @Princesseuh!

### Patch changes

- [a5b49ad](https://github.com/bruits/maudit/commit/a5b49adacbbbc18506e3157ade0547b60e35348a) Fixed an issue where Maudit would not properly liberate the port when shutting down — Thanks @Princesseuh!

## 0.6.2 — 2025-10-21

### Patch changes

- [6922bb3](https://github.com/bruits/maudit/commit/6922bb34bd42f1c1f1aa854f6ac77171b2c19ad0) Fixes fingerprinted assets reloading unnecessarily in development by introducing immutable cache headers on them — Thanks @Princesseuh!

## 0.6.1 — 2025-10-06

### Patch changes

- [9e3eb1e](https://github.com/bruits/maudit/commit/9e3eb1e15eec80377a38721406aae80f6a148f19) Fixes missing HTTP headers on HTML responses — Thanks @Princesseuh!

## 0.6.0 — 2025-10-06

### Minor changes

- [3758ccf](https://github.com/bruits/maudit/commit/3758ccfdcfbc66762d285d9f3bb2f7891b90dfe1) Improved stability of the watching system — Thanks @Princesseuh!
- [3758ccf](https://github.com/bruits/maudit/commit/3758ccfdcfbc66762d285d9f3bb2f7891b90dfe1) Fixes an issue where installation would fail due to an outdated Rolldown version — Thanks @Princesseuh!

## 0.5.1

### Patch changes

- [8f7edcc](https://github.com/bruits/maudit/commit/8f7edcc28898774c89408e9bc915f75cf483ee2f) Fixes hot reloading not working — Thanks @Princesseuh!


## 0.5.0

### Minor changes

- [2bfa8a8](https://github.com/bruits/maudit/commit/2bfa8a87212243b27c2231b836e7da9ec2cd3288) Improve general hot-reloading experience.
  
  The Maudit CLI will now output errors encountered during hot-reloading to the terminal and in the browser. — Thanks @Princesseuh!


## 0.4.5

### Patch changes

- [9cd5fdd](https://github.com/bruits/maudit/commit/9cd5fdd8abe3044bd09d48b96217e3a0d2878b13) Fixes missing DEV flag on rebuilds — Thanks @Princesseuh!


## 0.4.4

### Patch changes

- [6052fb8](https://github.com/bruits/maudit/commit/6052fb8dc2a6909477699d009d13bd193f06bc06) Pin tracing-subscriber to fix colored text output — Thanks @Princesseuh!
- [da95572](https://github.com/bruits/maudit/commit/da955725e460be405898b5749d64877404636e69) Fixed dev server reloading for no apparent reason on macOS — Thanks @Princesseuh!

## 0.4.1

### Patch changes

- [52eda9e](https://github.com/bruits/maudit/commit/52eda9ea4eac8efd3efd945d00f39a1b99f284ab) Improve performance — Thanks @Princesseuh!

