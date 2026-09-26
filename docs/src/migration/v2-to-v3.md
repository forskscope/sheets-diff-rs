# Migrating from 2.6.0 to 3.0

3.0 removes or changes public API that told callers something untrue: a
`SheetMatchReason` that claimed cell content was compared, four diagnostic codes that
could never arrive, options that could only fail, a count that was always zero. This
page maps each one to what you write instead.

**The baseline is 2.6.0.** If you are on an older 2.x, read the `CHANGELOG.md` entries
up to 2.6.0 first — the reorder exit code, the diagnostics counters and `--format json`
are 2.6.0's, and are not repeated here. Every example below is compiled and run when
the crate's docs are tested, so it cannot drift from the API it describes.

The one thing most likely to break your build is
[options are no longer built with a struct literal](#options-are-built-with-the-builder-not-a-struct-literal).
Start there.

---

## Quick reference

| 2.6.0 | 3.0 |
|---|---|
| `Limits { max_sheets: Some(50), ..Limits::default() }` (any options struct literal) | the builder, or `Default::default()` then assign — [details](#options-are-built-with-the-builder-not-a-struct-literal) |
| `to_json(&d)?`, `to_json_pretty(&d).unwrap()` | `to_json(&d)` — a `String`; delete the `?` |
| `CellAddress { .. }`, `ComparedRange { .. }` literals | `CellAddress::new`, `ComparedRange::empty` / `::union` |
| `SheetMatchReason::{ExactName, ContentSimilarity, IndexAndContent}` | `SameIndex` / `SoleRemainingPair` — **not a rename**, see [below](#sheetmatchreason) |
| four `DiagnosticKind` variants **and their code strings** | nothing — they could never occur — see [below](#four-diagnostickind-variants-and-their-code-strings) |
| `SourceKind::Unknown` | nothing |
| `DiffOptionsBuilder::number_compare` | `number_compare_policy` |
| `DiffOptionsBuilder::build_with_matching` | `.sheet_matching(..)` and/or `.alignment(..)` — and it fixes a silent bug |
| `FormulaCompareMode::{NormalizedText, RawAndNormalized}`, `FormatCompareMode::{Ignore, NumberFormatOnly, AllAvailable}` (the whole enum), `ComparisonOptions::format`, `format_compare` | nothing — they returned `InvalidOptions` when selected |
| `AlignmentMode::HeaderColumn` | `AlignmentMode::RowKey { columns: vec![1] }` — equivalent in 3.0.0; in 3.1.0 `RowKey` also compares rows with a blank key, so you may see more |
| `Severity::Error`, `DiagnosticSummary::errors`, the CLI's `N error(s)` | nothing — no diagnostic was ever fatal |
| `SheetsDiffError::{UnsupportedFormat, Internal}`, `OpenErrorKind::Locked` | `OpenWorkbook { kind: NotXlsx }` for the first; nothing for the others |

Also changed, and no compile error tells you: `DiffMetrics::cells_read` and
`Limits::max_cells_read` [count something else](#cells_read-and-max_cells_read-count-populated-cells);
`DiagnosticLocation` [is now filled in](#diagnosticlocation-names-the-sheet) where it was
`null`; and `CellAddress`'s ordering has [a qualification](#new-fields-are-additive-for-compilation-not-for-ordering).

---

## Changed

### Options are built with the builder, not a struct literal

**This is the break to expect, and we documented the pattern it breaks.** The API guide
showed `Limits { max_sheets: Some(50), ..Limits::default() }`, and a test pinned it as a
compatibility promise. If you followed our own guide, you are in this migration.

All eight options structs — `DiffOptions`, `ComparisonOptions`, `ValueCompareOptions`,
`MatchingOptions`, `Limits`, `ExecutionOptions`, `DiagnosticOptions`, `OutputOptions` —
are now `#[non_exhaustive]`, as every result struct always was. A struct expression
naming one no longer compiles from outside the crate:

```rust,compile_fail,E0639
use sheets_diff::Limits;

// 2.6.0 — the pattern our API guide showed. error[E0639]: cannot create non-exhaustive
// struct using struct expression
let _limits = Limits { max_sheets: Some(50), ..Limits::default() };
```

**`..Default::default()` does not help.** Functional update *is* a struct expression, so
it is rejected exactly as a full literal is:

```rust,compile_fail,E0639
use sheets_diff::{ComparisonOptions, ValueCompareOptions};

let _ = ComparisonOptions {
    value: ValueCompareOptions::default(),
    ..Default::default()
};
```

Use the builder, which covers every option, or `Default` followed by field assignment
(fields are still public):

```rust
use sheets_diff::{DiffOptions, Limits};

// The builder — preferred:
let built = DiffOptions::builder()
    .max_sheets(50)
    .max_cells_read(Some(1_000_000))
    .build()?;

// Or `Default`, then assign — what replaces a struct literal:
let mut assigned = DiffOptions::default();
assigned.limits.max_sheets = Some(50);
assigned.limits.max_cells_read = Some(1_000_000);

assert_eq!(built.limits.max_sheets, assigned.limits.max_sheets);
assert_eq!(built.limits.max_cells_read, assigned.limits.max_cells_read);

// A whole `Limits` value the same way:
let mut limits = Limits::default();
limits.max_sheets = Some(50);
assert_eq!(limits.max_sheets, Some(50));
# Ok::<(), sheets_diff::SheetsDiffError>(())
```

Why break every caller once: an option added later — RFC-022's format comparison, a
formula normaliser, column alignment — is now an added field, **not a breaking change**.
This is the last time constructing options is a break.

### `to_json` returns a `String`

`to_json` and `to_json_pretty` returned `Result<String, String>` and the `Err` could not
occur, so they now return `String`. This is the rare break that makes your code shorter:
**delete the `?`** (or the `.unwrap()`). The CLI's `--format json` output is unchanged.

```rust
# #[cfg(feature = "serde")]
# fn example() -> Result<(), sheets_diff::SheetsDiffError> {
use sheets_diff::compare_paths;
use sheets_diff::output::json::{to_json, to_json_pretty};

let diff = compare_paths(
    "tests/fixtures/generated/date_column/old.xlsx",
    "tests/fixtures/generated/date_column/new.xlsx",
)?;
// 2.6.0:  let json = to_json(&diff)?;
let json: String = to_json(&diff);
let pretty: String = to_json_pretty(&diff);
assert!(json.starts_with('{') && pretty.contains('\n'));
# Ok(())
# }
# #[cfg(feature = "serde")]
# example().unwrap();
```

### `CellAddress` and `ComparedRange` are built by their constructors

Both are `#[non_exhaustive]` now, for the same reason as the options. Build a
`CellAddress` with `CellAddress::new(row, col)` (1-based; `None` when out of range, and it
derives `a1` so the fields cannot disagree) and a `ComparedRange` with
`ComparedRange::empty()` or `ComparedRange::union(..)`. Reading their fields, and assigning
to a value you own, work as before.

```rust,compile_fail,E0639
use sheets_diff::CellAddress;

// 2.6.0. error[E0639]: cannot create non-exhaustive struct using struct expression
let _ = CellAddress { row: 3, col: 28, a1: "AB3".to_string() };
```

```rust
use sheets_diff::{CellAddress, ComparedRange};

let a = CellAddress::new(3, 28).expect("row 3, column 28 is in range");
assert_eq!((a.row, a.col, a.a1.as_str()), (3, 28, "AB3"));
assert!(CellAddress::new(0, 1).is_none());

let r = ComparedRange::union(Some((1, 1)), Some((2, 3)), Some((2, 2)), Some((5, 4)));
assert_eq!((r.start, r.end), (Some((1, 1)), Some((5, 4))));
assert_eq!(ComparedRange::empty().start, None);
```

### New fields are additive for compilation, not for ordering

`#[non_exhaustive]` makes a future field additive **for compilation**. It does not make it
behaviourally neutral. `CellAddress` derives `Ord`, and the derived order is field order —
`row`, `col`, `a1` — so a field added later would extend that ordering. Today the effect
is nil, because `a1` is derived from the other two. If you sort addresses, say what you
mean:

```rust
use sheets_diff::CellAddress;

let mut addrs = vec![
    CellAddress::new(2, 1).unwrap(),
    CellAddress::new(1, 3).unwrap(),
    CellAddress::new(1, 1).unwrap(),
];
// Sort by the documented `(row, col)` order rather than relying on the derived `Ord`.
addrs.sort_by_key(|a| (a.row, a.col));
let labels: Vec<&str> = addrs.iter().map(|a| a.a1.as_str()).collect();
assert_eq!(labels, ["A1", "C1", "A2"]);
```

### `SheetMatchReason`

Every rename in 2.6.0 carried `SheetMatchReason::IndexAndContent`, which says cell content
was compared. No cell content is ever compared: the matcher looks at sheet names, tab
positions, and which sheets are left over. The enum is now
`{ SameIndex, SoleRemainingPair }`:

- **`SameIndex`** — the two sheets sit at the same tab position and were paired on that.
- **`SoleRemainingPair`** — the names and positions both differ; each was simply the only
  unmatched sheet left. The weakest pairing the matcher makes.

**The migration is not a rename of the arm.** `IndexAndContent` was produced for *every*
rename, so an arm on it almost always meant "this is a rename". That question was never
answered by `reason`; it is answered by the `SheetChange`:

```rust
use sheets_diff::{SheetChange, SheetMatchReason, compare_paths};

let diff = compare_paths(
    "tests/fixtures/generated/renamed_sheet/old.xlsx",
    "tests/fixtures/generated/renamed_sheet/new.xlsx",
)?;

match &diff.sheets[0].change {
    // "Is this a rename?" — match the change, not the reason:
    SheetChange::Renamed { reason, .. } | SheetChange::RenamedAndMoved { reason, .. } => {
        // Use `reason` only to weigh how far to trust the pairing.
        assert_eq!(*reason, SheetMatchReason::SameIndex);
    }
    other => panic!("expected a rename, got {other:?}"),
}
# Ok::<(), sheets_diff::SheetsDiffError>(())
```

A caller who wrote `IndexAndContent => …` gets a compile error, which is intended: a
wildcard arm would have kept acting on a false premise. The serialised value changes too
(`"reason": "IndexAndContent"` becomes `"SameIndex"` or `"SoleRemainingPair"`). Which
sheets pair with which, and at what confidence, is unchanged. Here is the case that had no
true value before — two sheets paired only because each was the last one left, at
different positions:

```rust
use rust_xlsxwriter::Workbook;
use sheets_diff::{SheetChange, SheetMatchReason, compare_bytes};

fn workbook(sheets: &[&str]) -> Vec<u8> {
    let mut wb = Workbook::new();
    for name in sheets {
        let ws = wb.add_worksheet();
        ws.set_name(*name).unwrap();
        ws.write_string(0, 0, "x").unwrap();
    }
    wb.save_to_buffer().unwrap()
}

// old: A(0) B(1)    new: B(0) C(1).  `B` pairs by name; `A` and `C` are what is left.
let diff = compare_bytes(&workbook(&["A", "B"]), &workbook(&["B", "C"]))?;
let c = diff.sheets.iter().find(|s| s.new_sheet.as_ref().is_some_and(|r| r.name == "C")).unwrap();
match &c.change {
    SheetChange::RenamedAndMoved { reason, .. } => {
        assert_eq!(*reason, SheetMatchReason::SoleRemainingPair);
    }
    other => panic!("expected RenamedAndMoved, got {other:?}"),
}
# Ok::<(), sheets_diff::SheetsDiffError>(())
```

### `DiagnosticLocation` names the sheet

`location.sheet_order` and `location.sheet_name` are now set together, or both left
`null`, and `null` means **"not about a sheet"** rather than "not recorded". The two
sheet-level alignment warnings (`alignment_bound_exceeded`, `duplicate_alignment_key`) and
the sheet-visibility diagnostic used to leave the location empty (or half-filled); they now
name their sheet — for a renamed or moved sheet, the new workbook's. No type changed, so
this only shows up in what you read:

```rust
use rust_xlsxwriter::Workbook;
use sheets_diff::options::AlignmentMode;
use sheets_diff::{DiffOptions, compare_bytes_with_options};

fn workbook(rows: &[&str]) -> Vec<u8> {
    let mut wb = Workbook::new();
    let ws = wb.add_worksheet();
    ws.set_name("Data").unwrap();
    for (r, key) in rows.iter().enumerate() {
        ws.write_string(r as u32, 0, *key).unwrap();
    }
    wb.save_to_buffer().unwrap()
}

let opts = DiffOptions::builder()
    .alignment(AlignmentMode::RowKey { columns: vec![1] })
    .build()?;
// Two rows share the key "dup" on the old side.
let diff = compare_bytes_with_options(&workbook(&["dup", "dup", "u"]), &workbook(&["dup", "u"]), opts)?;

let warning = diff.sheets[0]
    .diagnostics
    .iter()
    .find(|d| d.kind.code() == "duplicate_alignment_key")
    .expect("the duplicate key is reported");
// 2.6.0: both were `None`.
assert_eq!(warning.location.sheet_name.as_deref(), Some("Data"));
assert_eq!(warning.location.sheet_order, Some(0));
# Ok::<(), sheets_diff::SheetsDiffError>(())
```

### `cells_read` and `max_cells_read` count populated cells

Through 2.6.0 both were the **area of the bounding box** of each sheet's populated cells:
the memory a dense read allocated, until 2.5.1 made the read stream and nothing spent it.
They now count **populated cells**. On the `sparse_range` fixture (two populated cells,
`A1` and `Z100`, per side) `cells_read` was 5,200 and is now 4, against `cells_compared`'s 2.
Anything that stored or asserted on the number sees the change (`--format json` publishes it
as `metrics.cells_read`); where every position inside a sheet's box is filled, nothing
moves.

**The bound only loosens.** A bounding box contains every cell in it, so the populated count
never exceeds the box area: *no workbook 2.6.0 accepted is refused now*. Some it refused are
accepted — chiefly a sparse box, a few cells far apart, which the old bound rejected and which
costs memory only in proportion to its cells. `LimitExceeded { limit: CellsRead, observed }`
now reports the running count at the breaking cell, so `observed` is `max + 1`. `hardened()`'s
value is unchanged. The threat model's [Sheet reading section](../maintainers/threat-model.md)
has the reasoning and the measurements.

```rust
use sheets_diff::{DiffOptions, compare_paths_with_options};

let opts = DiffOptions::builder().max_cells_read(Some(1_000)).build()?;
let diff = compare_paths_with_options(
    "tests/fixtures/generated/sparse_range/old.xlsx",
    "tests/fixtures/generated/sparse_range/new.xlsx",
    opts,
)?;
// 2.6.0 refused this under a bound of 1,000 (its 100 x 26 box is 2,600 per side).
assert_eq!(diff.metrics.cells_read, 4);
assert_eq!(diff.metrics.cells_compared, 2);
# Ok::<(), sheets_diff::SheetsDiffError>(())
```

---

## Removed

### `SheetMatchReason::{ExactName, ContentSimilarity, IndexAndContent}`

Replaced by `SameIndex` and `SoleRemainingPair` — and it is a change of meaning, not a rename;
see [`SheetMatchReason`](#sheetmatchreason) above. `ExactName` and `ContentSimilarity` were
never produced at all.

### Four `DiagnosticKind` variants, and their code strings

`DiagnosticKind::code()` is documented as the stable thing to match on, and four of the eleven
codes could never arrive. There is nothing to migrate *to* — delete the arms. **A caller
matching the code string gets no compile error and simply stops matching**, so search your own
code for these four strings:

| Variant | Code string |
|---|---|
| `DiagnosticKind::FormulaCachedValueUnverified` | `"formula_cached_value_unverified"` |
| `DiagnosticKind::UnsupportedCellValue { detail }` | `"unsupported_cell_value"` |
| `DiagnosticKind::DateTimeNotNormalized` | `"datetime_not_normalized"` |
| `DiagnosticKind::LimitTruncatedCells { limit, observed }` | `"limit_truncated_cells"` |

The seven codes that remain are each produced by a test. Matching on `code()` still works
exactly as before for them:

```rust
use sheets_diff::compare_paths;

let diff = compare_paths(
    "tests/fixtures/generated/chart_sheet/old.xlsx",
    "tests/fixtures/generated/chart_sheet/new.xlsx",
)?;
let dead = [
    "formula_cached_value_unverified",
    "unsupported_cell_value",
    "datetime_not_normalized",
    "limit_truncated_cells",
];
for d in diff.diagnostics.iter().chain(diff.sheets.iter().flat_map(|s| s.diagnostics.iter())) {
    assert!(!dead.contains(&d.kind.code()), "a removed code arrived: {}", d.kind.code());
}
assert!(diff.diagnostics.iter().any(|d| d.kind.code() == "unsupported_workbook_feature"));
# Ok::<(), sheets_diff::SheetsDiffError>(())
```

### `SourceKind::Unknown`

Nothing to migrate to: the crate builds every `SourceDescription` itself at one of three
entry points, so a source is always a `Path`, `Bytes` or `Reader`. `SourceKind` is
`#[non_exhaustive]`, so keep a wildcard arm — a future kind arrives as its own variant, not as
a fallback.

```rust
use sheets_diff::{SourceKind, compare_paths};

let diff = compare_paths(
    "tests/fixtures/generated/date_column/old.xlsx",
    "tests/fixtures/generated/date_column/new.xlsx",
)?;
let how = match diff.old.source.kind {
    SourceKind::Path => "a file on disk",
    SourceKind::Bytes => "bytes you held",
    SourceKind::Reader => "a reader",
    _ => "a source kind added after this was written",
};
assert_eq!(how, "a file on disk");
# Ok::<(), sheets_diff::SheetsDiffError>(())
```

### `DiffOptionsBuilder::number_compare`

It was `number_compare_policy` under a second name. Rename the call; the effect is identical.

```rust
use sheets_diff::{DiffOptions, NumberComparePolicy};

// 2.6.0:  .number_compare(NumberComparePolicy::AbsoluteTolerance(0.01))
let opts = DiffOptions::builder()
    .number_compare_policy(NumberComparePolicy::AbsoluteTolerance(0.01))
    .build()?;
assert_eq!(opts.comparison.value.number, NumberComparePolicy::AbsoluteTolerance(0.01));
# Ok::<(), sheets_diff::SheetsDiffError>(())
```

### `DiffOptionsBuilder::build_with_matching`

Chain `.sheet_matching(..)` and/or `.alignment(..)`, then `.build()`. **This is a bug fix as
well as a replacement.** `build_with_matching` assigned the whole `MatchingOptions`, so it
silently discarded a `sheet_matching` set earlier in the chain — if you wrote
`.sheet_matching(ExactNameOnly).build_with_matching(..)`, your code ended with the default and
may have been losing a setting. The chained setters keep both, in either order:

```rust
use sheets_diff::options::AlignmentMode;
use sheets_diff::{DiffOptions, SheetMatchingMode};

// 2.6.0:  .sheet_matching(ExactNameOnly)
//         .build_with_matching(MatchingOptions { sheet_matching: Default::default(),
//                                                alignment: AlignmentMode::RowKey { columns: vec![1] } })
//         — ended with the DEFAULT sheet matching, not `ExactNameOnly`.
let opts = DiffOptions::builder()
    .sheet_matching(SheetMatchingMode::ExactNameOnly)
    .alignment(AlignmentMode::RowKey { columns: vec![1] })
    .build()?;
assert_eq!(opts.matching.sheet_matching, SheetMatchingMode::ExactNameOnly);
assert!(matches!(opts.matching.alignment, AlignmentMode::RowKey { .. }));
# Ok::<(), sheets_diff::SheetsDiffError>(())
```

### `FormulaCompareMode::{NormalizedText, RawAndNormalized}`, `FormatCompareMode`, `ComparisonOptions::format`, `format_compare`

There is nothing to migrate to. `validate()` rejected all of them unconditionally, so a caller
who selected one received `Err(InvalidOptions)` and nothing else — remove the call.
`FormulaCompareMode` keeps `RawText` and `Ignore`, which work. The whole `FormatCompareMode`
enum — `Ignore`, `NumberFormatOnly` and `AllAvailable` — the `format` field and the
`format_compare` setter are gone, because that option could only ever hold `Ignore`: the other
two variants always failed, and a single-variant enum would have kept a public option whose only
usable setting is the one you get by not setting it.

They may return: RFC-022 (styles and formatting) is not withdrawn, and a formula normaliser
is a possibility. Because the options structs are `#[non_exhaustive]` from 3.0, adding an option
back is **not a breaking change**. Note also that no combination of options is currently
invalid, so `build()` cannot fail today; it still returns a `Result`, and
`SheetsDiffError::InvalidOptions` remains, for the next option that can be set to something
unusable.

```rust
use sheets_diff::{DiffOptions, FormulaCompareMode};

for mode in [FormulaCompareMode::RawText, FormulaCompareMode::Ignore] {
    DiffOptions::builder().formula_compare(mode).build()?;
}
# Ok::<(), sheets_diff::SheetsDiffError>(())
```

### `AlignmentMode::HeaderColumn`

It was exactly `AlignmentMode::RowKey { columns: vec![1] }` under another name: it delegated to
row-key alignment on column 1 and never read a header. Its name promised column alignment from
header names, which this crate does not have. Replace it with the `RowKey` form.

**In 3.0.0 the result is identical.** **In 3.1.0 it may not be, and that is a
fix.** `HeaderColumn` — and `RowKey` — never compared a row that had no cell in
the key column: the change in such a row was reported as no change, under
`confidence: Exact`. 3.1.0 compares them. So a migrated caller gets the
correction as well as the rename: identical keyless rows pair and stay silent,
but a *changed* keyless row is reported as a removal plus an insertion, and
`confidence` is capped at `Medium` whenever any row had no key. A sheet in which
every row has a key is unaffected.

```rust
use sheets_diff::options::AlignmentMode;
use sheets_diff::{DiffOptions, compare_paths_with_options};

// 2.6.0:  .alignment(AlignmentMode::HeaderColumn)
let opts = DiffOptions::builder()
    .alignment(AlignmentMode::RowKey { columns: vec![1] })
    .build()?;
let diff = compare_paths_with_options(
    "tests/fixtures/generated/alignment_header_column/old.xlsx",
    "tests/fixtures/generated/alignment_header_column/new.xlsx",
    opts,
)?;
let al = diff.sheets[0].alignment_summary.as_ref().expect("alignment ran");
// the header row plus the three original data rows match; one row was inserted
assert_eq!((al.matched_rows, al.inserted_rows, al.removed_rows), (4, 1, 0));
# Ok::<(), sheets_diff::SheetsDiffError>(())
```

### `Severity::Error`, `DiagnosticSummary::errors`, and the CLI's `N error(s)`

No diagnostic the engine produces is an error: a condition that stops a comparison is a
`SheetsDiffError` and produces no result, and one that does not is a `Diagnostic` at `Info` or
`Warning`. So the summary's error count was `0` on every run for every workbook. It is gone,
along with `Severity::Error` and the `"ERROR"` prefix in the unified output. There is nothing
to migrate to; delete the reads. Two visible effects for anything that parses output: the CLI's
summary line `diagnostics: 0 error(s), 2 warning(s)` is now `diagnostics: 2 warning(s)` (still
absent when there are no warnings), and `--format json`'s `summary.diagnostics` loses `errors`
(`{"errors":0,"warnings":2,"info":5}` becomes `{"warnings":2,"info":5}`).

`Severity` is `#[non_exhaustive]`, so keep a wildcard arm, and `Info < Warning` still holds, so
`min_severity` works as before:

```rust
use sheets_diff::{Severity, compare_paths};

let diff = compare_paths(
    "tests/fixtures/generated/chart_sheet/old.xlsx",
    "tests/fixtures/generated/chart_sheet/new.xlsx",
)?;
let d = &diff.summary.diagnostics;
// 2.6.0 also had `d.errors`, always 0.
assert_eq!((d.warnings, d.info), (2, 5));
assert!(Severity::Info < Severity::Warning);
for x in &diff.diagnostics {
    let label = match x.severity {
        Severity::Info => "info",
        Severity::Warning => "warning",
        _ => "a severity added after this was written",
    };
    assert!(label == "info" || label == "warning");
}
# Ok::<(), sheets_diff::SheetsDiffError>(())
```

### `SheetsDiffError::{UnsupportedFormat, Internal}` and `OpenErrorKind::Locked`

None of the three was ever constructed. A non-`.xlsx` file is, and always was,
`OpenWorkbook { kind: NotXlsx }` — that is what to match — and it still exits `3` from the CLI.
`Internal` was an escape hatch for our own bugs that was never needed. `Locked` named a real
condition (a file another program holds open) that this crate never detected; such a file
arrives as `PermissionDenied` or `Other`. **No exit code changed.** Keep a wildcard arm: both
enums are `#[non_exhaustive]`.

```rust
use sheets_diff::{OpenErrorKind, SheetsDiffError, compare_paths};

let err = compare_paths(
    "tests/fixtures/corrupt/not_a_zip.xlsx",
    "tests/fixtures/generated/date_column/new.xlsx",
)
.unwrap_err();
match err {
    // 2.6.0 code matching `SheetsDiffError::UnsupportedFormat { .. }` never ran; this is the real one.
    SheetsDiffError::OpenWorkbook { kind: OpenErrorKind::NotXlsx, .. } => {}
    other => panic!("expected OpenWorkbook/NotXlsx, got {other}"),
}
```

---

## What did not change

A guide that only lists breaks reads as though everything moved. It did not:

- **Comparison results are unchanged.** Which cells differ, which sheets pair and at what
  confidence, and every diagnostic produced (per-scenario counts included) are the same. Over the
  19-scenario corpus, the whole `--format json` result of 2.6.0 and 3.0 differ **only** in the
  fields named on this page: `metrics.cells_read` (3 scenarios), `summary.diagnostics.errors`
  (removed everywhere), and `reason` on a rename.
- **The six `compare_*` entry points are untouched**, and so is every other function not
  named here.
- **The serialised shape is unchanged** apart from those fields, plus `location.sheet_order` /
  `location.sheet_name` on the diagnostics listed above. No field or variant name outside the
  removals was renamed.
- **The CLI's exit codes are unchanged** for every input class, including under `--format json`.
- **The minimum supported Rust version is still 1.88.**
- **Field assignment still works** on every options struct, and `Default` is unchanged for all of
  them.
