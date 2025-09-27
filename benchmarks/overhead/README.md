# overhead-benchmark

This crate contains a Maudit website that generates 10000 pages with no content to benchmark the overhead of Maudit itself.

## Running the benchmark

To run the benchmark, execute the following command:

```sh
cargo bench
```

## Results

The following results were obtained on 2025-09-27 using a MacBook Pro (13-inch, M1, 2020) with 16 GB of RAM:

| Median Full Build Time |
| ---------------------- |
| 1.164s                 |

These numbers are not scientific and only serve as a rough estimate of the performance of Maudit. Your mileage may vary.
