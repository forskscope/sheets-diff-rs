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

## A golden's first bless is the one moment its content is unreviewed

A golden only detects *change* — it cannot detect having been born wrong.
Every bless after the first compares new output against a value someone
already reviewed; the first bless has nothing to compare against but
itself, so "the test passes" is not evidence that its content is correct.

This is not hypothetical. The `formula` scenario existed since RFC-015 to
test formula-versus-value changes. Its first bless recorded the `=1+1` →
`=2+0` change at cell **A1** — the cell containing the text label `"label"`
— with two spurious `FormulaUnavailable` diagnostics at **A2**, where the
formula actually was. `A1` had no formula at all. That golden was wrong
from the day it was blessed and stayed wrong through every subsequent run
for the same reason it was never caught: a passing test only proves output
hasn't *changed*, and this output never changed — it was consistently
wrong. RFC-035 Handoff 05's D-04 fix (a value-range/formula-range
coordinate-translation bug in `read_sheet_cells`) moved it to `A2`, where
the formula actually is.

**So: blessing a new scenario means reading the produced `expected.json`
and deciding it is right** — every address, every diagnostic, every
count — **not observing that `cargo test` then passes.** The "read the
resulting diff before committing it" instruction in Step 2 above applies
with extra force on a scenario's *first* bless, because there is no prior
diff to have already been checked.

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
