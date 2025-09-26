# Benchmarks

This directory contains various benchmarks for Maudit.

## On compile times

All the numbers in these benchmarks only include the **running time** of the benchmark. [Maudit operates on the idea that your content and assets change way more often than any parts that would require re-compilation](https://maudit.org/docs/philosophy/#your-website-changes-less-often-than-its-content) (static templates, pretty much anything in a `*.rs` file) and as such expect that the vast majority of your builds won't require compilation.

This is not a gotcha moment or anything we're trying to hide: **With compilation times included, Maudit is slower than most static site generators.**
