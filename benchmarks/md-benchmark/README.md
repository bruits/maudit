# md-benchmark

This crate contains a Maudit website with presets of various amount of markdown files and is used to benchmark the performance of Maudit's Markdown rendering.

The generated Markdown files were taken from https://github.com/zachleat/bench-framework-markdown. Thanks to Zach Leatherman for providing the benchmark data and comparaison points with other static site generators.

## Running the benchmark

To run the benchmark, execute the following command:

```sh
cargo bench
```

5 benchmarks with different number of pages (250, 500, 1000, 2000, 4000) will be run and the time for each benchmark will be printed to the console.

## Results

The following results were obtained on 2025-08-27 using a MacBook Pro (13-inch, M1, 2020) with 16 GB of RAM:

| Pages | Median Full Build Time (ms) |
| ----- | --------------------------- |
| 250   | 37                          |
| 500   | 75                          |
| 1000  | 151                         |
| 2000  | 319                         |
| 4000  | 676                         |

These numbers are not scientific and only serve as a rough estimate of the performance of Maudit. Your mileage may vary.
