# Comparison semantics

Five scenarios, each run for real against a fixture already committed to
this repository's test corpus (`tests/fixtures/generated/`) — every
`assert_eq!` below is checked by CI on every push, not copied from a
description of what the engine is supposed to do. None of these examples
need a new file or a dev-dependency: they read the same corpus
`tests/`/`examples/gen-fixtures.rs` already maintain, which is why they can
run rather than merely compile (see the [API guide](api-guide.md) for
examples that need a workbook this page's doctests don't have — those are
`no_run`; these are not).

---

## Typed value change

A single cell's value changes; nothing else in the workbook does.

```rust
let diff = sheets_diff::compare_bytes(
    &std::fs::read("tests/fixtures/generated/sparse_range/old.xlsx").unwrap(),
    &std::fs::read("tests/fixtures/generated/sparse_range/new.xlsx").unwrap(),
).unwrap();

assert_eq!(diff.summary.cells_changed, 1);
assert_eq!(diff.summary.values_changed, 1);
assert_eq!(diff.summary.formulas_changed, 0);

let cell = &diff.sheets[0].cell_diffs[0];
assert_eq!(cell.address.a1, "A1");

let value_change = cell.value.as_ref().unwrap();
assert_eq!(value_change.old, sheets_diff::CellValue::Text("A1".into()));
assert_eq!(value_change.new, sheets_diff::CellValue::Text("A1_changed".into()));
assert_eq!(value_change.reason, sheets_diff::ValueDifferenceKind::ContentChanged);
```

**What the caller observes:** `CellDiff.value` is `Some(ValueChange)`;
`CellDiff.formula` stays `None`. `ValueChange.reason` names *why* the two
sides differ — here, `ContentChanged` (same type, different content).
`DiffSummary.values_changed` counts this cell; `formulas_changed` does not.

---

## Formula change

Formula text changes to an equivalent expression (`=1+1` → `=2+0`) — the
*value* both formulas would produce is the same, but the formula's own
text differs, and this engine compares formula text, never evaluates it.

```rust
let diff = sheets_diff::compare_bytes(
    &std::fs::read("tests/fixtures/generated/formula/old.xlsx").unwrap(),
    &std::fs::read("tests/fixtures/generated/formula/new.xlsx").unwrap(),
).unwrap();

assert_eq!(diff.summary.cells_changed, 1);
assert_eq!(diff.summary.values_changed, 0);
assert_eq!(diff.summary.formulas_changed, 1);

let cell = &diff.sheets[0].cell_diffs[0];
assert!(cell.value.is_none());

let formula_change = cell.formula.as_ref().unwrap();
assert_eq!(formula_change.old.as_ref().unwrap().raw, "1+1");
assert_eq!(formula_change.new.as_ref().unwrap().raw, "2+0");
```

**What the caller observes:** `CellDiff.formula` is `Some(FormulaChange)`
while `CellDiff.value` stays `None` — formula and value changes are
independent sub-fields of the same `CellDiff`, and a cell can carry either,
both, or neither. `DiffSummary.formulas_changed` counts this cell;
`values_changed` does not, because nothing about the cached *value*
changed — only what produces it.

---

## Sheet rename

A sheet's name changes between the two workbooks, with one cell inside it
also changing.

```rust
let diff = sheets_diff::compare_bytes(
    &std::fs::read("tests/fixtures/generated/renamed_sheet/old.xlsx").unwrap(),
    &std::fs::read("tests/fixtures/generated/renamed_sheet/new.xlsx").unwrap(),
).unwrap();

let sheet = &diff.sheets[0];
assert_eq!(sheet.old_sheet.as_ref().unwrap().name, "OldName");
assert_eq!(sheet.new_sheet.as_ref().unwrap().name, "NewName");
match &sheet.change {
    sheets_diff::SheetChange::Renamed { confidence, .. } => {
        assert_eq!(*confidence, sheets_diff::MatchConfidence::Medium);
    }
    other => panic!("expected Renamed, got {other:?}"),
}
```

**What the caller observes:** `SheetDiff.change` is
`SheetChange::Renamed { confidence, reason }` — rename detection is
conservative and heuristic (this project's design note: "only fires when
exactly one old and one new sheet are unmatched"), so `confidence` is worth
checking before treating a rename as certain in a UI. `old_sheet`/
`new_sheet` still carry both names, and the sheet's own `cell_diffs` are
unaffected by the rename — a renamed sheet's cell changes are reported
exactly as an unrenamed sheet's would be.

---

## Inserted row

**The single most surprising behaviour in this engine**, and the one most
likely to be mistaken for a bug: whether an inserted row cascades into
every row below it depends entirely on `AlignmentMode`, which defaults to
`Positional`.

```rust
use sheets_diff::{DiffOptions, compare_bytes, compare_bytes_with_options};
use sheets_diff::options::{AlignmentMode, MatchingOptions, SheetMatchingMode};

let old = std::fs::read("tests/fixtures/generated/alignment_row_signature/old.xlsx").unwrap();
let new = std::fs::read("tests/fixtures/generated/alignment_row_signature/new.xlsx").unwrap();

// Positional (the default): row N on the old side is compared against
// row N on the new side, period. A row inserted above existing data shifts
// every subsequent row's position, so *every* row after the insertion
// point reports as changed -- not because its content changed, but
// because a different row's content now occupies that position.
let positional = compare_bytes(&old, &new).unwrap();
assert_eq!(positional.summary.cells_changed, 12);

// RowSignature: rows are matched by a content signature rather than
// position, so a row that moved (because something was inserted above it)
// is still recognised as "the same row, unmoved" -- only the genuinely
// new row reports as a change.
let opts = DiffOptions::builder()
    .build_with_matching(MatchingOptions {
        sheet_matching: SheetMatchingMode::default(),
        alignment: AlignmentMode::RowSignature { sample_columns: None },
    })
    .unwrap();
let aligned = compare_bytes_with_options(&old, &new, opts).unwrap();
assert_eq!(aligned.summary.cells_changed, 2);

let summary = aligned.sheets[0].alignment_summary.as_ref().unwrap();
assert_eq!(summary.inserted_rows, 1);
assert_eq!(summary.matched_rows, 5);
```

**Same two files. 12 changed cells under the default mode; 2 under
`RowSignature`.** Both are correct — they answer different questions.
`Positional` answers "what changed at each row position"; `RowSignature`
(and `RowKey`, keyed on a specific column instead of a whole-row content
hash) answer "what changed to each logical row, wherever it now sits."
Choosing wrong for the data doesn't produce an error — it produces a
result that is technically accurate and practically misleading. There is
no way to detect which the caller wanted; `Positional` is default because
it's the only mode requiring no assumption about the data's shape.

`row_insertion_cascade` in the same corpus shows the same effect at a
larger scale (42 cells cascade under `Positional`, one real insertion) —
not repeated here since the mechanism is identical.

---

## Warning handling

A comparison can succeed — return `Ok(WorkbookDiff)` — while also carrying
diagnostics about things it could not fully compare. This is not an error
path; it's the normal path for input this engine partially supports.

```rust
let diff = sheets_diff::compare_bytes(
    &std::fs::read("tests/fixtures/generated/chart_sheet/old.xlsx").unwrap(),
    &std::fs::read("tests/fixtures/generated/chart_sheet/new.xlsx").unwrap(),
).unwrap();

// The comparison succeeded and found a real cell change...
assert_eq!(diff.summary.cells_changed, 1);

// ...while also reporting that a chart sheet's content was not compared.
assert_eq!(diff.summary.diagnostics.warnings, 2);
assert_eq!(diff.summary.diagnostics.info, 1);

let warning = diff
    .diagnostics
    .iter()
    .find(|d| d.severity == sheets_diff::Severity::Warning)
    .unwrap();
assert_eq!(warning.kind.code(), "unsupported_workbook_feature");
assert!(warning.message.contains("chart sheet"));
```

**What the caller observes:** `WorkbookDiff.diagnostics` (and each
`SheetDiff.diagnostics`) carries entries independent of whether the
comparison found any cell differences — `Ok` says the comparison
completed, not that everything was compared. Two `Warning`-severity
diagnostics appear here (one emitted per side, since the chart sheet
exists — unchanged — in both the old and new workbook) plus one `Info`
diagnostic that is emitted on **every** comparison unconditionally: a
blanket coverage note listing everything this engine never compares
(charts, images, comments, data validation, conditional formatting;
hyperlinks, merged regions, tables, and pivot tables — see
[non-goals and limitations](non-goals.md) for which of those are upstream
gaps and which are simply not implemented yet). `DiagnosticKind::code()`
is the stable, programmatic identifier — this crate's own GUI-embedding
consumer matches on it rather than the human-readable `message`.
