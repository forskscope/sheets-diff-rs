# Changelog

## [2.1.0] - 2026-06-11

### Added

- **RFC-011 — Row alignment** (`AlignmentMode::RowKey`, `RowSignature`):
  opt-in row matching by key columns or content signature to reduce
  false-positive cascades after row insertions/deletions.
  `SheetDiff.alignment_summary` is populated when alignment is active.
- **RFC-021 — Workbook metadata diffs**: defined-name additions, removals, and
  target changes are reported as `Info`-severity diagnostics in
  `WorkbookDiff.diagnostics`. Sheet visibility changes are similarly reported.
  Defined-name scope is unavailable in calamine 0.35; a
  `DefinedNameScopeUnknown` diagnostic is attached when names are present.
- **RFC-022 — Format comparison policy**: `FormatCompareMode` enum added to
  `ComparisonOptions`. Selecting anything other than `Ignore` returns
  `SheetsDiffError::InvalidOptions` — calamine 0.35 exposes no cell-style API
  and the policy is honest about that.
- **RFC-029 — GUI view adapters** (`output::view`): `DiffView`, `CellChangeRow`,
  `SheetSummaryRow`, `ChangeAnchor`, `ViewFilter`. Framework-neutral borrowed
  iterators for sheet-tree, flat change-list, and prev/next navigation.
- `DiffOptionsBuilder::build_with_matching` convenience method.
- `FormatCompareMode` re-exported from crate root.

## [2.0.1] - 2026-06-11

### Added

- Expanded integration test corpus covering all RFC-015 fixture categories:
  corrupt inputs, wide-column A1 encoding (A–XFD), typed-value distinctions,
  formula handling, sheet rename/add/remove, empty and sparse ranges, resource
  limits, progress events, cancellation, text output, and JSON output.
- `tests/support.rs` — shared programmatic fixture builders.
- `tests/fixtures/corrupt/not_a_zip.xlsx` — committed corrupt binary fixture.
- `docs/src/migration/v1-to-v2.md` — migration guide (RFC-017): entry points,
  sheet changes, cell value model, duplicate-address policy, errors,
  diagnostics, text output, CLI exit codes, and a v1-style flattening helper.
- `docs/src/SUMMARY.md` and `docs/src/README.md` — mdbook scaffolding.

### Changed

- `compare` module is now `pub` so integration tests can call
  `compare_values_pub` directly; the function is `#[doc(hidden)]`.

## [2.0.0] - 2026-06-11

Complete rewrite.  v2 is a structured, library-first `.xlsx` diff engine.

### Breaking changes from v1

- **New public types**: `WorkbookDiff`, `SheetDiff`, `CellDiff`, `CellValue`
  replace the old `Diff`/`SheetDiff`/`CellDiff` string model.
- **Typed cell values**: `CellValue::Integer`, `Number`, `Bool`, `DateTime`,
  `Duration`, `Error`, `Empty` — no more stringly-typed old/new fields.
- **One `CellDiff` per address**: value and formula changes are subfields
  (`value`, `formula`), not separate entries.
- **Structured errors**: `SheetsDiffError` is `#[non_exhaustive]`; no more
  panics on ordinary bad input.
- **No stdout/stderr writes** from library code.
- Entry points: `compare_paths`, `compare_bytes`, `compare_readers` (and
  `_with_options` variants).

### New features

- Conservative sheet rename detection (`SheetMatchingMode::ExactNameThenConservativeRename`).
- `DiffOptions` grouped tree with builder; `Limits`, `ProgressSink`,
  `Cancellation` hooks.
- `EncryptedWorkbook` error for password-protected files.
- Correct Excel A1 addressing through column `XFD` (column 16 384).
- Deterministic result ordering by sheet index, then `(row, col)`.
- Text and unified-diff output formatters over `WorkbookDiff`.
- Optional `serde` feature: `Serialize` derives on all public model types.

### Migration from v1

See `docs/migration/v1-to-v2.md` (RFC-017 deliverable, to be added).

Quick reference:

| v1 | v2 |
|---|---|
| `Diff::new(old, new)` | `compare_paths(old, new)?` |
| `diff.cell_diffs[i].old` (String) | `diff.sheets[s].cell_diffs[c].value.as_ref().map(|v| v.old.display_string())` |
| `CellDiffKind::Value / Formula` | `CellDiff.value.is_some()` / `.formula.is_some()` |
| panic on bad input | `Err(SheetsDiffError::...)` |
