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

Adding or changing a scenario is **two explicit steps**, and both are
reviewed like any other change — neither is a silent, automatic pass.

**Step 1 — regenerate `old.xlsx` / `new.xlsx` / `scenario.toml`.** Produced
by an example, not by the test suite (RFC-034 Handoff 01) — `cargo test`
never writes into this directory, and neither does this example write
`expected.json`; see "A generator that also blesses is a bug" below.

```sh
cargo run --example gen-fixtures
```

Every generated workbook carries a fixed creation timestamp, so re-running
the generator against *unchanged* scenario definitions reproduces the same
bytes. That makes a silent `git status --porcelain tests/fixtures/generated`
after this step meaningful for one narrow question only — "did I just
regenerate an unchanged scenario by mistake?" — not "is everything fine."
It says nothing about `expected.json`, which this step never touches.

**Step 2 — bless `expected.json`.** The *only* thing that writes or updates a
golden is:

```sh
BLESS=1 cargo test --features serde -- generated_fixtures_match_golden
```

This asserts the new output against the existing golden first; `BLESS=1`
only takes effect for scenarios that don't already match (or don't yet have
an `expected.json`). **Read the resulting diff of `expected.json` before
committing it** — that diff is the regression check for this scenario, and
skimming past it defeats the corpus's purpose.

## Scenario metadata

Each scenario directory contains a `scenario.toml`:

```toml
name        = "wide_columns_xfd"
kind        = "regression"        # regression | feature | edge_case
description = "Covers A1 addressing through column XFD (column 16384)."
notes       = ""
```

## A generator that also blesses is a bug

`examples/gen-fixtures.rs` does not depend on `sheets-diff`'s comparison
logic and cannot write `expected.json`, by design. A generator that both
produces fixtures *and* silently rewrites their goldens defeats regression
protection at exactly the moment it matters: change comparison behaviour,
regenerate, and the goldens would rewrite themselves to match the new
(possibly wrong) behaviour before anyone compares against the old one. If a
future change reintroduces golden-writing into the generator, that is a
regression against RFC-034's explicit prohibition ("do not make blessing
implicit, or on-mismatch-rewrite") — treat it as a bug, not a convenience.

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
