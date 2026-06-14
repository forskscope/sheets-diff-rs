# Changelog

## [2.2.3] - 2026-06-11

### Fixed (audit)

- **Dead code removed:**
  - `OpenedWorkbook::sheet_names()` was never called; removed.
  - `AlignmentModeLabel` enum and `AlignmentSummaryData.mode` field were
    written but never read; removed.
  - `make_renamed_workbook` in `benches/workbook_diff.rs` was unused; removed.
  - Crate-level `#![allow(dead_code)]` removed — no longer needed.
- **Metrics corrected:** `DiffMetrics.cells_read` now reflects the actual cell
  count from `read_sheet_cells` (was `1` per sheet). `DiffMetrics.cells_compared`
  now counts all coordinate pairs visited, not just changed cells.
- **`compare` module made `pub(crate)`** — it is internal machinery. The
  `compare_values_pub` test helper is now `#[cfg(test)]` only.
- **Stale doc comments updated:** `WorkbookChange` / `WorkbookObjectChange` /
  `WorkbookDiff` comments no longer reference "v2.0" or "always empty in v2.0";
  they correctly describe the v2.2 state (RFC-021/023 surface through
  `diagnostics`; structured variants reserved for future).
- **`criterion::black_box` deprecation** resolved — switched to
  `std::hint::black_box` throughout `benches/workbook_diff.rs`.

### Added (audit)

- `#[non_exhaustive]` added to all 26 public model types that were missing it
  (RFC-031 compliance).
- `CellDisplay::new()` and `CellSnapshot::new()` constructors — necessary
  because `#[non_exhaustive]` blocks struct literal construction outside the
  crate.
- `DiffOptionsBuilder::number_compare_policy()` builder method.
- Integration tests for `compare_readers` / `compare_readers_with_options`
  (RFC-004, previously untested) and `TypeMismatchPolicy::CompareDisplayString`
  (RFC-010, previously untested).

## [2.2.2] - 2026-06-11

### Changed

- Updated `criterion` from `0.5` to `0.8` (latest).
- Moved `criterion` from `[dependencies]` (optional) to `[dev-dependencies]`
  where it belongs — it is a benchmarking tool and has no place in the
  published dependency tree. The `bench` feature flag is removed; benches
  now compile unconditionally with `cargo build --benches`.
- Fixed two pre-existing bugs in `benches/workbook_diff.rs` that were
  previously hidden behind `required-features = ["bench"]`: a lifetime
  error in `bench_many_sheets` and a stale variable reference in
  `bench_alignment_vs_positional`.

## [2.2.1] - 2026-06-11

Additive response to integration feedback from ForskScope. No breaking changes.

### Added

- `output::view::CellChangeRow` now carries `old_formula: Option<&str>` and
  `new_formula: Option<&str>`, borrowed from the underlying `CellDiff`. GUI
  consumers can render formula changes without reaching past the view layer
  into the raw model.
- `output::view::OwnedCellChangeRow` — a fully owned counterpart to
  `CellChangeRow`, plus `CellChangeRow::to_owned_row()`. Convenience for
  consumers whose model outlives the `WorkbookDiff`.
- `ChangeAnchor` now derives `serde::Serialize` (under the `serde` feature).

### Documentation

- `Cancellation` trait: added an `Arc<AtomicBool>` cancellation example and a
  "Cancellation latency" section documenting that `is_cancelled()` is polled
  once per sheet pair (not mid-sheet).
- `DiagnosticKind::code()`: documented as the stable programmatic surface for
  diagnostics, with a full table of the current code strings.
- `CellDiff`: documented the "one `CellDiff` per address" consumer model and
  confirmed `change_kind()`'s derivation as stable API.
- `compare_paths`: documented that non-UTF-8 paths are fully supported with no
  internal `to_str()`/`unwrap()` on the path.
- `WorkbookDiff`: documented that `summary`, `metrics`, and the per-sheet
  `change` list are cheap to extract so bulky `cell_diffs` can be dropped.

## [2.2.0] - 2026-06-11

### Added

- **RFC-023 — Object / unsupported-feature coverage diagnostics**: every
  comparison emits an `Info`-level `UnsupportedWorkbookFeature` diagnostic
  explaining that charts, images, comments, hyperlinks, tables, pivot tables,
  and data validation are not compared. Non-worksheet sheet types (ChartSheet,
  MacroSheet, VBA) emit a `Warning`. Controlled by `ObjectCompareMode` (default
  `WarnIfPresent`); suppressible via `DiffOptionsBuilder::object_mode(Ignore)`.
- **RFC-024 — `DiffMetrics`**: `WorkbookDiff.metrics` carries `sheets_read`,
  `cells_read`, `cells_compared`, `diffs_emitted`, and `diagnostics_emitted`
  for benchmarking and performance analysis.
- **RFC-025 — Parallel sheet comparison** (`parallel` feature, off by default):
  `ExecutionMode::Parallel` processes sheets in parallel with `rayon`, then
  sorts results by original workbook order to guarantee identical output.
  Enable with `--features parallel`; select via
  `DiffOptionsBuilder::execution_mode(ExecutionMode::Parallel)`.
- **RFC-027 — Benchmarks** (`bench` feature): `benches/workbook_diff.rs`
  covers all eight RFC-027 scenarios (small-business, wide, tall, sparse,
  many-sheets, formula, rename, alignment cascade). Run with
  `cargo bench --features bench`.
- **RFC-028 — Fuzz targets** (`fuzz/`): four `cargo-fuzz` targets covering
  `compare_bytes` on arbitrary input, `col_to_label` roundtrip,
  `ComparedRange::union`, and `DiffOptionsBuilder::build`. Corpus seeds in
  `fuzz/corpus/fuzz_open_xlsx_bytes/`. See `fuzz/README.md`.
- **RFC-020 — Display formatting types**: `CellDisplay`, `CellSnapshot`,
  `CellNumberFormat`, `DisplaySource` added to the public model. `CellDisplay`
  carries a deterministic display string, an optional number-format record
  (`None` in calamine 0.35 — reserved for RFC-022), and a `DisplaySource` tag.
  `CellSnapshot` groups a `CellValue`, optional `FormulaText`, and optional
  `CellDisplay` with a `preferred_display()` helper. `CellValue::display_default()`
  is an alias for `display_string()` as per RFC-020 §6.
- **RFC-030 — Extended fixture corpus**: `tests/gen.rs` generates seven scenario
  fixtures (wide_columns, renamed_sheet, typed_values, formula, empty_sheet,
  sparse_range, row_insertion_cascade) into `tests/fixtures/generated/`, each
  with a `scenario.toml` and (with `--features serde`) an `expected.json`
  golden file. `tests/fixtures/corpus/README.md` documents the contribution policy.
- `ComparedRange::union` made `pub` (was `pub(crate)`).
- `DiffOptionsBuilder::object_mode`, `::execution_mode`, `::format_compare`
  builder methods.

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
