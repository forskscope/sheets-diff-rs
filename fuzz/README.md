# Fuzzing `sheets-diff`

Fuzz targets are in `fuzz/src/` and use [cargo-fuzz](https://github.com/rust-fuzz/cargo-fuzz)
with libFuzzer.

## Prerequisites

```sh
cargo install cargo-fuzz
```

## Running a target

```sh
# From the repository root:
cargo fuzz run fuzz_open_xlsx_bytes
cargo fuzz run fuzz_addr_roundtrip
cargo fuzz run fuzz_range_merge
cargo fuzz run fuzz_diff_options_builder
```

## Available targets

| Target | What it tests |
|---|---|
| `fuzz_open_xlsx_bytes` | `compare_bytes` on arbitrary input — must never panic |
| `fuzz_addr_roundtrip` | `col_to_label` and `CellAddress::new` consistency |
| `fuzz_range_merge` | `ComparedRange::union` on arbitrary coordinate pairs |
| `fuzz_diff_options_builder` | `DiffOptionsBuilder::build` on arbitrary option combos |

## Corpus seeds

`fuzz/corpus/fuzz_open_xlsx_bytes/` contains seed inputs: empty file, random
bytes, and a truncated ZIP header. Add any crashing inputs found during fuzzing
to the appropriate corpus directory.

## Panic policy

Public APIs must not panic on malformed input (RFC-028 §7). Any panic found
via fuzzing is a bug. File an issue with the minimized corpus entry and open a
PR referencing it.

## CI

Normal CI compiles the fuzz targets but does not run them (would be too slow).
Run fuzzing manually or in a nightly workflow with a time budget, e.g.:

```sh
cargo fuzz run fuzz_open_xlsx_bytes -- -max_total_time=300
```
