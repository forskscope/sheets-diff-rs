# `sheets-diff` Test Fixture Corpus

This directory documents the structure of the generated fixture corpus and
explains how to contribute a minimal reproducer.

## Structure

```
tests/fixtures/
  generated/           programmatic fixtures: old.xlsx + new.xlsx + scenario.toml
    wide_columns/
    renamed_sheet/
    typed_values/
    formula/
    empty_sheet/
    sparse_range/
    row_insertion_cascade/
  corrupt/             malformed binary fixtures
  corpus/              this README
```

## Generating fixtures

Fixtures in `generated/` are produced by the test suite itself using
`rust_xlsxwriter`. Run:

```sh
cargo test -- generate_fixtures
```

This writes `old.xlsx`, `new.xlsx`, and (when `--features serde` is active)
`expected.json` into the appropriate subdirectory.

## Scenario metadata

Each scenario directory contains a `scenario.toml`:

```toml
name        = "wide_columns_xfd"
kind        = "regression"        # regression | feature | edge_case
description = "Covers A1 addressing through column XFD (column 16384)."
notes       = ""
```

`expected.json` is the golden serialised `WorkbookDiff` when the `serde`
feature is enabled. Regenerate with:

```sh
cargo test --features serde -- bless_fixtures
```

## Contributing a reproducer

1. Reduce the problem to the smallest `old.xlsx` / `new.xlsx` pair that
   reproduces it.
2. Generate the pair programmatically in `tests/support.rs` if possible, or
   include the binary files if the workbook structure cannot be reproduced with
   `rust_xlsxwriter`.
3. Add a `scenario.toml` with `kind = "regression"` and a reference to the
   issue or PR that motivated the fix.
4. Do **not** include real customer data. All fixtures must be fully synthetic
   or explicitly sanitised.
