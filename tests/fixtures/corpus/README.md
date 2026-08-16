# `sheets-diff` Test Fixture Corpus

This directory documents the structure of the generated fixture corpus and
explains how to contribute a minimal reproducer.

## Structure

```
tests/fixtures/
  generated/           programmatic fixtures: old.xlsx + new.xlsx + scenario.toml
                        (+ expected.json for all but one — see the coverage
                        matrix below for the full list and what each covers)
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
BLESS=1 cargo test --features serde,chrono -- generated_fixtures_match_golden
```

`serde` and `chrono` together, not `serde` alone, are the canonical feature
set goldens are blessed and compared under (RFC-036 Handoff 02 correction
C-01): `CellDateTime.iso` is populated only when `chrono` is enabled, so a
date-bearing fixture's exact JSON depends on it. Gating the exact-match
check on both features together means exactly one CI leg (`serde,chrono,cli`)
ever performs it, so no golden can be correct on one leg and wrong on
another — the `serde`-only leg still runs the error-free comparison and
every hand-written assertion in this file, which are feature-invariant by
construction, but not the exact-JSON check.

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

## Coverage matrix (RFC-036)

**Covered** means: an assertion exists that would fail if the behaviour for
that dimension broke. A fixture that merely *contains* a pattern is not
coverage — see the section above. Where a scenario's correct answer is
knowable independently of the fixture generator itself, the golden alone is
not enough either; there is a dedicated assertion in `tests/integration.rs`
for every row below, not just a golden comparison.

| # | Scenario (directory) | Closes | Assertion |
|---|---|---|---|
| 1 | `row_shifted_origin` | origin not at A1, row axis | `row_shifted_origin_fixture_reports_correct_address` |
| 2 | `formula_shifted_origin` + `formula_at_first_cell` | general case of D-04's origin fix; the negative control that would have hidden D-04 entirely | `formula_shifted_origin_fixture_attaches_to_the_real_formula_cell`, `formula_at_first_cell_fixture_negative_control` |
| 3 | `alignment_row_signature` | `AlignmentMode::RowSignature` — zero coverage before this | `alignment_row_signature_fixture_reduces_cascade` |
| 4 | `alignment_header_column` | `AlignmentMode::HeaderColumn` — zero coverage before this | `alignment_header_column_fixture_reduces_cascade` |
| 5 | `error_values` | `CellError` comparison and `ValueDifferenceKind::ErrorKindChanged` — zero coverage at any level before this | `error_values_fixture_detects_error_kind_change` |
| 6 | `sheet_reordered` | `SheetChange::Moved`, never distinguished from `Unchanged` by any prior assertion | `sheet_reordered_fixture_distinguishes_moved_from_modified` |
| 7 | `date_column` | no golden-corpus fixture used dates at all, despite dates being where four M2 defects lived | `date_column_fixture_detects_date_change`, plus the golden — blessed under the canonical `serde,chrono` feature set (see Step 2 above) since `CellDateTime.iso` is `chrono`-conditional |
| 8 | `non_ascii_text` | zero coverage of non-ASCII sheet names / cell text through the shared-string table | `non_ascii_text_fixture_detects_change` |
| 9 | `chart_sheet` | the chart-sheet coverage diagnostic had never fired in any test | `chart_sheet_fixture_fires_diagnostic_and_compares_the_worksheet` |
| 10 | `empty_cell_before_content` | a physically-present-but-empty leading `<c>` element must not anchor the range origin (confirmed against calamine's actual behaviour, not just its source) | `empty_cell_before_content_fixture_does_not_anchor_origin` |
| 11 | `iso_datetime` | promotes RFC-035 Handoff 05's hand-built ISO-datetime reachability test into a durable corpus trip-wire | `iso_datetime_fixture_detects_change` |

Scenarios 1–9 are pure `rust_xlsxwriter` output (RFC-036 Handoff 02); 10 and
11 are generated then XML-patched via `patch_xlsx_xml`, duplicated into
`examples/gen-fixtures.rs` from `tests/support.rs` for the same
generator-independence reason the other builders there are duplicated
(Handoff 03 §1) — because `rust_xlsxwriter` has no way to emit either
pattern (a physically-present empty cell nothing was written to; a `t="d"`
ISO-typed cell).

Full derivation, including the two findings that are *not* fixture gaps —
`CellValue::Integer`/`Duration`/`Unsupported` cannot occur through any
`.xlsx` input at all, and the `<dimension>` XML tag turned out not to be a
hazard calamine's range bounds ever consult — is in
[RFC-036](../../../rfcs/accepted/036-coverage-obligation-and-the-fixture-matrix.md)
and the coverage-dimension report it was built from.

## The coverage obligation (RFC-036 §5.3)

**A change to `normalize.rs`, `compare.rs`, `align.rs`, or `diff.rs` that
alters behaviour for a dimension in the matrix above must arrive with an
assertion for that dimension, or state in its review request why none is
needed.**

This is a review-time obligation, not an automated gate — "did this change
need a fixture?" is a question a reviewer must see answered, not something
CI can reliably decide on its own. It exists because this project has
already had the alternative twice: a code path acquiring behaviour that
nothing checked, once for D-04 (a pattern present in a fixture for over a
year with no assertion on it) and once for the four dimensions this matrix
was built to close (`SheetChange::Moved`, `CellError` comparison, and the
two alignment modes with zero prior coverage — see the table above).

New dimensions get added to the matrix when found, not deferred to a future
audit — this table is the living record RFC-036 describes, not a snapshot.
A dimension may be **explicitly deferred** with a stated reason; an
undocumented gap is a defect, a documented one is a decision.

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
