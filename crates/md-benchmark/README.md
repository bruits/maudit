# md-benchmark

This crate contains a Maudit website with presets of various amount of markdown files and is used to benchmark the performance of Maudit.

The generated Markdown files were taken from https://github.com/zachleat/bench-framework-markdown, in order to make somewhat fair comparisons with other static site generators.

## Running the benchmark

To run the benchmark, execute the following command:

```sh
cargo run --release
```

By default, this will build 1000 pages. You can change the number of pages to build by using the `MARKDOWN_COUNT` environment variable:

```sh
MARKDOWN_COUNT=4000 cargo run --release
```

Valid values for `MARKDOWN_COUNT` are 250, 500, 1000, 2000, and 4000.

## Results

The following results were obtained on 2025-01-07 using a MacBook Pro (13-inch, M1, 2020) with 16 GB of RAM:

| Pages | Full Build Time (ms) |
| ----- | -------------------- |
| 250   | 55                   |
| 500   | 113                  |
| 1000  | 253                  |
| 2000  | 504                  |
| 4000  | 922                  |

These numbers are not scientific and only serve as a rough estimate of the performance of Maudit. Your mileage may vary.

Note also that those numbers do not include the compilation time of the project, which can be significant for large projects.
