# Changelog

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
