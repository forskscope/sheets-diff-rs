# Changelog

All notable changes to `sheets-diff` are documented here.

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
Versions follow [Semantic Versioning](https://semver.org/).

---

## [1.2.0] — 2026-06-11

Stabilization release for field-reported `.xlsx` comparison defects
found during GUI integration. All v1.1.4 callers remain source-compatible.

### Added

- **`Diff::try_new(impl AsRef<Path>, impl AsRef<Path>) -> Result<Diff, SheetsDiffError>`**
  Fallible path-based constructor. Returns a structured error instead of
  panicking on missing, corrupt, locked, or non-`.xlsx` inputs. Accepts
  `&str`, `String`, and `PathBuf` without requiring `.to_str().unwrap()`.
  ([RFC-002](rfcs/done/002-fallible-diff-construction-error-model.md),
   [RFC-004](rfcs/done/004-reader-based-and-path-safe-input-constructors.md))

- **`Diff::try_from_named_readers(name, R, name, R) -> Result<Diff, SheetsDiffError>`**
  Reader-based constructor for GUI and VCS tools that already hold workbook
  bytes. Accepts any `Read + Seek` stream (e.g. `std::io::Cursor<Vec<u8>>`).
  The supplied names populate `Diff.old_filepath` / `Diff.new_filepath`.
  ([RFC-004](rfcs/done/004-reader-based-and-path-safe-input-constructors.md))

- **`SheetsDiffError` error type** (`src/core/error.rs`)
  Structured error with variants `OpenWorkbook`, `OpenReader`,
  `ReadSheetValues`, `ReadSheetFormulas`. Each variant carries the
  `WorkbookSide` (`Old`/`New`), the path or sheet name, and the
  originating `calamine::XlsxError` as a source. Implements `Debug`,
  `Display`, and `std::error::Error`.
  ([RFC-002](rfcs/done/002-fallible-diff-construction-error-model.md))

- **`col_to_label(col: usize) -> String`** (`src/core/utils.rs`)
  Public helper: converts a 1-based column index to its Excel label.
  ([RFC-003](rfcs/done/003-full-excel-a1-addressing-and-ordering.md))

- RFC directory (`rfcs/`) with lifecycle policy and all v1.2.0 design
  documents. ([RFC-000](rfcs/done/000-rfc-lifecycle-policy.md))

### Fixed

- **A1 address generation for wide worksheets** — The old implementation
  cast the column index to `u8`, silently truncating columns above 255 and
  potentially underflowing at column 256 in debug builds. The new
  implementation uses base-26 arithmetic with `usize` throughout. Column
  16,384 now correctly produces `XFD`.
  ([RFC-003](rfcs/done/003-full-excel-a1-addressing-and-ordering.md))

- **Cell diff sort order** — Cell diffs are now sorted by numeric
  `(row, col, kind)` rather than by lexical A1 string, which placed `A10`
  before `A2`. Output ordering is now consistent with spreadsheet grid order.
  ([RFC-003](rfcs/done/003-full-excel-a1-addressing-and-ordering.md))

- **Spurious empty-formula diff** — v1.1.4 emitted a diff entry when one
  workbook had no formula for a cell and the other had an empty-string
  formula. Both are now treated as equivalent.

- **Library stdout writes removed** — `println!("Failed to read sheet: …")`
  calls have been removed from library code. Sheet read failures are
  propagated as `SheetsDiffError` through the fallible API.
  ([RFC-005](rfcs/done/005-library-diagnostics-without-stdout-writes.md))

### Changed

- **`Diff::new` now panics with a diagnostic message** on open/read
  failure, delegating to `try_new(...).expect(...)`. The runtime behavior
  is the same (a panic), but the panic message now includes the structured
  error description. Existing callers are not affected.
  ([RFC-002](rfcs/done/002-fallible-diff-construction-error-model.md))

- **CLI updated** to use `Diff::try_new` and print a human-readable error
  message to stderr with `process::exit(2)` instead of panicking.
  ([RFC-005](rfcs/done/005-library-diagnostics-without-stdout-writes.md))

- **Crate edition** updated from `2021` to `2024`.

- **`rust-version`** bumped from `1.78.0` to `1.85.0` (minimum required
  for the 2024 edition).

### Internal

- Extracted `Diff::empty`, `Diff::try_from_workbooks`,
  `Diff::collect_diff_from_workbooks`, and `Diff::normalize_cell_diffs`
  private helpers. All public constructors share one diff engine with no
  duplicated logic.
- `collect_cell_value_diff` and `collect_cell_formula_diff` are now
  generic over `R: Read + Seek` and return `Result<(), SheetsDiffError>`.

### Tests

- Added `tests/utils_address.rs` — 4 unit tests for A1 label generation
  and numeric sort order.
- Added `tests/constructors.rs` — 11 integration tests for all constructors
  and error paths.
- Added `tests/wide_columns.rs` — 6 integration tests for columns IV, IW,
  AAA, and XFD.
- Added `tests/fixtures/wide-columns-old.xlsx` and
  `tests/fixtures/wide-columns-new.xlsx` (generated via `rust_xlsxwriter`).
- Added `tests/fixtures/non-xlsx.txt` for negative-path tests.
- `[dev-dependencies]` adds `rust_xlsxwriter = "0"`.
- Updated golden-fixture expectation to reflect corrected output.

---

## [1.1.4] — prior release

See the v1.1.4 source archive for history before this changelog was
introduced.
