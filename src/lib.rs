#![forbid(unsafe_code)]
// M5 Handoff 01/04 (RFC-016, RFC-005, RFC-013): library core must not write
// to stdout/stderr. `forbid`, not `deny` -- `deny` can be overridden by an
// inner `#[allow(clippy::disallowed_macros)]`, which is exactly the bypass
// this closes (clippy's own `-D warnings` failure message suggests that
// override; `forbid` turns the attempt into a compile error instead). The
// scoped gate (`.github/clippy-no-stdout/clippy.toml`, loaded via
// `CLIPPY_CONF_DIR` for `cargo clippy --lib`) supplies the actual macro/method
// paths; with no config loaded elsewhere, these two lints have nothing
// configured and are free everywhere else in the tree.
#![forbid(clippy::disallowed_macros, clippy::disallowed_methods)]

//! # sheets-diff
//!
//! Structured diff engine for Microsoft Excel `.xlsx` workbooks.
//!
//! ## Quick start
//!
//! ```rust,no_run
//! use sheets_diff::compare_paths;
//!
//! let diff = compare_paths("old.xlsx", "new.xlsx")?;
//! println!("changed cells: {}", diff.summary.cells_changed);
//! # Ok::<(), sheets_diff::SheetsDiffError>(())
//! ```
//!
//! ## Input sources
//!
//! | Function | When to use |
//! |---|---|
//! | [`compare_paths`] | Simplest; caller provides file paths |
//! | [`compare_bytes`] | You already have the bytes (e.g. from a cache or repo) |
//! | [`compare_readers`] | You have open `Read + Seek` handles |
//! | `compare_*_with_options` variants | Any of the above plus [`DiffOptions`] |
//!
//! See [`DiffOptions`] / [`DiffOptionsBuilder`] for all configuration knobs.

// ---------------------------------------------------------------------------
// Internal modules (not pub)
// ---------------------------------------------------------------------------

pub mod address;
mod align;
mod diff;
mod error;
mod matcher;
mod meta;
mod normalize;
mod objects;
mod open;

pub(crate) mod compare;

// ---------------------------------------------------------------------------
// Public modules
// ---------------------------------------------------------------------------

/// Typed result model (`WorkbookDiff`, `SheetDiff`, `CellDiff`, `CellValue`, …).
pub mod model;

/// Comparison options and builder (`DiffOptions`, `DiffOptionsBuilder`, …).
pub mod options;

/// Output formatters (text summary, unified diff).
pub mod output;

// ---------------------------------------------------------------------------
// Re-exports — the stable public API surface (RFC-002, RFC-031)
// ---------------------------------------------------------------------------

// Error types
pub use error::{LimitKind, OpenErrorKind, ReadErrorKind, SheetsDiffError};

// Model
pub use model::{
    AlignmentSummary, CellChangeKind, CellDateTime, CellDiff, CellDisplay, CellDuration, CellError,
    CellNumberFormat, CellSnapshot, CellValue, DateTimeKind, Diagnostic, DiagnosticKind,
    DiagnosticLocation, DiagnosticSummary, DiffMetrics, DiffStage, DiffSummary, DisplaySource,
    FormatChange, FormulaChange, FormulaText, MatchConfidence, Severity, SheetChange, SheetDiff,
    SheetMatchReason, SheetRef, SheetSummary, Side, SourceDescription, SourceKind, ValueChange,
    ValueDifferenceKind, WorkbookChange, WorkbookDiff, WorkbookObjectChange, WorkbookSideInfo,
};

// Address
pub use address::{CellAddress, ComparedRange, MAX_COL, MAX_COL_LABEL, MAX_ROW};

// Options
pub use objects::ObjectCompareMode;
pub use options::{
    AlignmentMode, Cancellation, ComparisonOptions, DateComparePolicy, DiagnosticOptions,
    DiffEvent, DiffOptions, DiffOptionsBuilder, ExecutionMode, ExecutionOptions, FormatCompareMode,
    FormulaCompareMode, Limits, MatchingOptions, NumberComparePolicy, NumericTypePolicy,
    OutputOptions, ProgressSink, SheetMatchingMode, TypeMismatchPolicy, ValueCompareOptions,
};

// ---------------------------------------------------------------------------
// Public entry points (RFC-033 §12)
// ---------------------------------------------------------------------------

use std::io::{Read, Seek};
use std::path::Path;

/// Compare two workbooks given their filesystem paths.
///
/// Uses [`DiffOptions::default()`].
/// Compare two workbooks given their filesystem paths.
///
/// # Path handling
///
/// `old` and `new` accept any `AsRef<Path>`, and the raw `Path` is passed to
/// `std::fs::read` unchanged — there is **no internal `to_str()`/`unwrap()` on
/// the path**, so non-UTF-8 paths (common on Linux) are fully supported and
/// never cause a panic. The only UTF-8-dependent step is the cosmetic
/// `SourceDescription.display_name`, which is set to `None` for a non-UTF-8
/// file name rather than failing.
pub fn compare_paths(
    old: impl AsRef<Path>,
    new: impl AsRef<Path>,
) -> Result<WorkbookDiff, SheetsDiffError> {
    diff::run_compare_paths(old, new, DiffOptions::default())
}

/// Compare two workbooks given their filesystem paths, with explicit options.
pub fn compare_paths_with_options(
    old: impl AsRef<Path>,
    new: impl AsRef<Path>,
    opts: DiffOptions,
) -> Result<WorkbookDiff, SheetsDiffError> {
    diff::run_compare_paths(old, new, opts)
}

/// Compare two workbooks given byte slices.
pub fn compare_bytes(
    old: impl AsRef<[u8]>,
    new: impl AsRef<[u8]>,
) -> Result<WorkbookDiff, SheetsDiffError> {
    diff::run_compare_bytes(old, new, DiffOptions::default())
}

/// Compare two workbooks given byte slices, with explicit options.
pub fn compare_bytes_with_options(
    old: impl AsRef<[u8]>,
    new: impl AsRef<[u8]>,
    opts: DiffOptions,
) -> Result<WorkbookDiff, SheetsDiffError> {
    diff::run_compare_bytes(old, new, opts)
}

/// Compare two workbooks given `Read + Seek` readers.
///
/// `.xlsx` is ZIP-based and requires seeking.
pub fn compare_readers<R1, R2>(old: R1, new: R2) -> Result<WorkbookDiff, SheetsDiffError>
where
    R1: Read + Seek,
    R2: Read + Seek,
{
    diff::run_compare_readers(old, new, DiffOptions::default())
}

/// Compare two workbooks given `Read + Seek` readers, with explicit options.
pub fn compare_readers_with_options<R1, R2>(
    old: R1,
    new: R2,
    opts: DiffOptions,
) -> Result<WorkbookDiff, SheetsDiffError>
where
    R1: Read + Seek,
    R2: Read + Seek,
{
    diff::run_compare_readers(old, new, opts)
}

// ---------------------------------------------------------------------------
// Documentation doctest harness (M6 Handoff 01, NF-025)
// ---------------------------------------------------------------------------
//
// `include_str!`-ing a `docs/` page as this item's doc comment turns every
// ```rust fence in it into a doctest `cargo test --doc` compiles — the same
// mechanism, and the same guarantee, as any other doc example in this file.
// `#[cfg(doctest)]` means the item exists only while doctests are being
// collected, so it is absent from every normal build (`cargo build`,
// `cargo doc`, `cargo clippy`) and costs nothing there.
//
// To add a future page to this harness: add one more
// `#[doc = include_str!("../docs/src/<path>.md")] #[cfg(doctest)]` item
// below, following this one.
#[doc = include_str!("../docs/src/migration/v1-to-v2.md")]
#[cfg(doctest)]
pub struct MigrationGuideDoctests;

#[doc = include_str!("../docs/src/api-guide.md")]
#[cfg(doctest)]
pub struct ApiGuideDoctests;

#[doc = include_str!("../docs/src/semantics.md")]
#[cfg(doctest)]
pub struct SemanticsDoctests;

#[doc = include_str!("../docs/src/non-goals.md")]
#[cfg(doctest)]
pub struct NonGoalsDoctests;
