# RFC 002 — Fallible Diff Construction and Error Model

**Status.** Implemented (v1.2.0)
**Tracks.** No-panic library integration.
**Touches.** `src/core/error.rs` (new), `src/core/diff.rs`, `src/core/mod.rs`,
`src/main.rs`, `tests/constructors.rs`.

## Summary

Added `Diff::try_new(...) -> Result<Diff, SheetsDiffError>` and introduced
the `SheetsDiffError` structured error type. The existing
`Diff::new(&str, &str) -> Diff` remains as a panicking convenience wrapper.

## Problem

The v1.1.4 implementation opened workbooks inside `collect_diff()` with
`expect(...)`. Any missing, corrupt, locked, password-protected, or
non-`.xlsx` input could panic the calling thread — unacceptable for GUI
embedders.

## Delivered API

```rust
// src/core/error.rs
pub enum SheetsDiffError {
    OpenWorkbook { side: WorkbookSide, path: PathBuf, source: calamine::XlsxError },
    OpenReader   { side: WorkbookSide, source: calamine::XlsxError },
    ReadSheetValues   { side: WorkbookSide, sheet: String, source: calamine::XlsxError },
    ReadSheetFormulas { side: WorkbookSide, sheet: String, source: calamine::XlsxError },
}

pub enum WorkbookSide { Old, New }
```

`SheetsDiffError` implements `Debug`, `Display`, and `std::error::Error`.

```rust
// Fallible path constructor
impl Diff {
    pub fn try_new(
        old_filepath: impl AsRef<Path>,
        new_filepath: impl AsRef<Path>,
    ) -> Result<Self, SheetsDiffError>;
}

// Convenience panicking wrapper (unchanged contract)
impl Diff {
    pub fn new(old_filepath: &str, new_filepath: &str) -> Self {
        match Self::try_new(old_filepath, new_filepath) {
            Ok(diff) => diff,
            Err(err) => panic!("failed to diff workbooks: {err}"),
        }
    }
}
```

## Internal architecture

Introduced private helpers:

- `Diff::empty(old_label, new_label)` — builds an empty `Diff`.
- `Diff::try_from_workbooks(...)` — shared engine used by all constructors.
- `Diff::collect_diff_from_workbooks(...)` — returns `Result<(), SheetsDiffError>`.
- `Diff::normalize_cell_diffs()` — merge and sort, extracted from `new`.

## CLI

Updated to use `Diff::try_new` with human-readable `eprintln!` on error
and `process::exit(2)`.

## Open questions

None.
