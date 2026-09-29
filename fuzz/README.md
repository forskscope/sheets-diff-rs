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
cargo fuzz run fuzz_self_comparison
cargo fuzz run fuzz_addr_roundtrip
cargo fuzz run fuzz_range_merge
cargo fuzz run fuzz_diff_options_builder
```

## Available targets

| Target | What it tests |
|---|---|
| `fuzz_open_xlsx_bytes` | `compare_bytes` on arbitrary input — must never panic |
| `fuzz_self_comparison` | `compare_bytes_with_options` on a workbook compared with itself, under a fuzz-driven `AlignmentMode` and a bounded `Limits` — must produce zero cell diffs and no sheet change other than `Unchanged`, not just "did not panic" |
| `fuzz_addr_roundtrip` | `col_to_label` and `CellAddress::new` consistency |
| `fuzz_range_merge` | `ComparedRange::union` on arbitrary coordinate pairs |
| `fuzz_diff_options_builder` | `DiffOptionsBuilder::build` on arbitrary option combos |

## Corpus seeds (M9 unit 01)

Both `fuzz_open_xlsx_bytes` and `fuzz_self_comparison` take a **framed** seed — a small fixed header
in front of the actual fuzz payload — not raw bytes. The framing functions
(`fuzz/src/framing.rs`, `fuzz/src/self_seed.rs`) are `include!`d by the target, by
`tests/fuzz_corpus_reaches_reader.rs` (the permanent guard, in the ordinary gate — it fails if either
corpus stops reaching the reader) and by `examples/gen-fuzz-corpus.rs` (the generator), so writing a
seed by hand means using `make_seed`/`make_self_seed`, not concatenating bytes directly.

- **`fuzz/corpus/fuzz_open_xlsx_bytes/`** — a 4-byte little-endian length prefix for `old`, then
  `old`'s bytes, then `new`. `empty`, `random_bytes` and `truncated_zip` are the original hand-written
  seeds (still valid framing; they just decode to inputs that fail to open). The `paired_*` seeds are
  generated from `tests/fixtures/generated/*/{old,new}.xlsx` and cover RFC-028 §6's ten categories —
  see the RFC's own §6 annotation for which seed is which.
- **`fuzz/corpus/fuzz_self_comparison/`** — a 3-byte header (`AlignmentMode` and a bounded `Limits`
  override, see `fuzz/src/self_seed.rs`), then one workbook, compared with itself.
  `self_blank_key_rows_*` is the regression seed for f130's original class of defect: a blank id in
  every 20th row, under `RowKey`, `Positional` and `RowSignature` as controls.

Regenerate both with `cargo run --example gen-fuzz-corpus`; every generated workbook pins a fixed
creation timestamp, so this reproduces the committed corpus byte-for-byte. Add a crashing input to
`fuzz/artifacts/<target>/` (gitignored — see below) as evidence, not to `fuzz/corpus/`; a corpus seed
is meant to be read back by name, and a raw crash artifact is not.

`cargo fuzz run`, `cargo fuzz coverage` and `cargo fuzz tmin` all write into `fuzz/target/`,
`fuzz/artifacts/` and `fuzz/coverage/` as a side effect of running — the first was already gitignored;
the other two were not, so a local fuzzing session could leave untracked binary files one `git add -A`
away from being committed. All three are gitignored now (`.gitignore`).

## Panic policy

Public APIs must not panic on malformed input (RFC-028 §7). Any panic found
via fuzzing is a bug. File an issue with the minimized corpus entry and open a
PR referencing it.

## CI

**One seed is quarantined and one target is not in the `fuzz-smoke` matrix — see
`fuzz/corpus-quarantine/README.md`.** f132 and f133 each closed a defect that used to put something
there; both stay out anyway, for one remaining, still-open `calamine` defect that is not either
unit's to fix — see the quarantine README.

Normal CI compiles the fuzz targets but does not run them (would be too slow).
Run fuzzing manually or in a nightly workflow with a time budget, e.g.:

```sh
cargo fuzz run fuzz_open_xlsx_bytes -- -max_total_time=300
```
