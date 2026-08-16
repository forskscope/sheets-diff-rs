//! Public result data model.
//!
//! All types here are normatively defined in RFC-033.  This module owns
//! construction and the summary / change-kind derivation logic; the field
//! shapes are fixed by the canonical lexicon.

use std::fmt;

#[cfg(feature = "serde")]
use serde::Serialize;

use crate::address::{CellAddress, ComparedRange};

// ---------------------------------------------------------------------------
// Side
// ---------------------------------------------------------------------------

/// Which workbook of the pair a piece of data refers to.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize))]
#[non_exhaustive]
pub enum Side {
    Old,
    New,
}

impl fmt::Display for Side {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Side::Old => f.write_str("old"),
            Side::New => f.write_str("new"),
        }
    }
}

// ---------------------------------------------------------------------------
// Source description
// ---------------------------------------------------------------------------

/// What kind of input source a workbook came from.
#[derive(Clone, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize))]
#[non_exhaustive]
pub enum SourceKind {
    Path,
    Bytes,
    Reader,
    Unknown,
}

/// Caller-visible description of a workbook input source.
///
/// `display_name` is never an absolute path unless the caller explicitly
/// provided it as such.
#[derive(Clone, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize))]
#[non_exhaustive]
pub struct SourceDescription {
    pub kind: SourceKind,
    pub display_name: Option<String>,
}

// ---------------------------------------------------------------------------
// Per-side workbook metadata
// ---------------------------------------------------------------------------

#[derive(Clone, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize))]
#[non_exhaustive]
pub struct WorkbookSideInfo {
    pub source: SourceDescription,
    pub workbook_name: Option<String>,
    pub sheet_count: usize,
}

// ---------------------------------------------------------------------------
// Sheet identity
// ---------------------------------------------------------------------------

/// A reference to a specific sheet in one workbook.
///
/// `index` is **0-based** workbook order (as returned by calamine).
#[derive(Clone, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize))]
#[non_exhaustive]
pub struct SheetRef {
    pub name: String,
    pub index: usize,
}

// ---------------------------------------------------------------------------
// Sheet change classification (RFC-009 / RFC-033 §6)
// ---------------------------------------------------------------------------

/// How confident the sheet-matching algorithm is about a non-exact pairing.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize))]
#[non_exhaustive]
pub enum MatchConfidence {
    Exact,
    High,
    Medium,
    Low,
}

/// The reason a non-exact sheet pair was formed.
#[non_exhaustive]
#[derive(Clone, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub enum SheetMatchReason {
    ExactName,
    IndexAndContent,
    ContentSimilarity,
}

/// How a sheet pair was classified.
///
/// Names and indices live in `SheetDiff.old_sheet` / `SheetDiff.new_sheet`;
/// they are **not** duplicated inside the variant payloads.
#[non_exhaustive]
#[derive(Clone, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub enum SheetChange {
    /// Name-matched, index unchanged, no cell differences.
    Unchanged,
    /// Name-matched (or rename-matched), has cell differences.
    Modified,
    /// New sheet with no counterpart in the old workbook.
    Added,
    /// Old sheet with no counterpart in the new workbook.
    Removed,
    /// Name-matched, but the tab index moved between the two workbooks.
    Moved,
    /// Name changed; heuristically matched.
    Renamed {
        confidence: MatchConfidence,
        reason: SheetMatchReason,
    },
    /// Both renamed and moved.
    RenamedAndMoved {
        confidence: MatchConfidence,
        reason: SheetMatchReason,
    },
}

// ---------------------------------------------------------------------------
// CellValue and components (RFC-007 / RFC-033 §2–§3)
// ---------------------------------------------------------------------------

/// Spreadsheet-serial date/time value captured from calamine.
///
/// `serial` is the Excel date serial (days since 1900-01-00 or 1904-01-01).
/// `is_1904` distinguishes the two date systems.
/// `iso` is populated when calamine provides an ISO string directly or when the
/// `chrono` feature can synthesize one.
#[derive(Clone, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize))]
#[non_exhaustive]
pub struct CellDateTime {
    pub serial: f64,
    pub is_1904: bool,
    pub kind: DateTimeKind,
    pub iso: Option<String>,
}

/// Whether an Excel date serial represents a date, time, or datetime.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize))]
#[non_exhaustive]
pub enum DateTimeKind {
    DateTime,
    Date,
    Time,
}

/// Spreadsheet-serial duration value (ISO 8601 duration string when available).
#[derive(Clone, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize))]
#[non_exhaustive]
pub struct CellDuration {
    pub serial: f64,
    pub iso: Option<String>,
}

/// Typed spreadsheet cell error.
///
/// Maps 1-to-1 with calamine's `CellErrorType`; `Other` handles forward-compat.
#[non_exhaustive]
#[derive(Clone, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub enum CellError {
    Div0,
    NA,
    Name,
    Null,
    Num,
    Ref,
    Value,
    GettingData,
    Other(String),
}

impl fmt::Display for CellError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CellError::Div0 => f.write_str("#DIV/0!"),
            CellError::NA => f.write_str("#N/A"),
            CellError::Name => f.write_str("#NAME?"),
            CellError::Null => f.write_str("#NULL!"),
            CellError::Num => f.write_str("#NUM!"),
            CellError::Ref => f.write_str("#REF!"),
            CellError::Value => f.write_str("#VALUE!"),
            CellError::GettingData => f.write_str("#GETTING_DATA"),
            CellError::Other(s) => write!(f, "#{s}"),
        }
    }
}

/// Typed representation of a spreadsheet cell value (RFC-033 §2).
///
/// `Integer` and `Number` are kept distinct (reflecting calamine's `Data::Int`
/// / `Data::Float`).  Default comparison treats `Integer(1)` vs `Number(1.0)`
/// as a `TypeChanged` difference; cross-type numeric equality is opt-in
/// (RFC-019).
#[non_exhaustive]
#[derive(Clone, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub enum CellValue {
    Empty,
    Text(String),
    Integer(i64),
    Number(f64),
    Bool(bool),
    DateTime(CellDateTime),
    Duration(CellDuration),
    Error(CellError),
    Unsupported { display: String, reason: String },
}

impl CellValue {
    /// A human-readable display string.  For use in reports only; never used
    /// as an equality key.
    pub fn display_string(&self) -> String {
        match self {
            CellValue::Empty => String::new(),
            CellValue::Text(s) => s.clone(),
            CellValue::Integer(i) => i.to_string(),
            CellValue::Number(f) => f.to_string(),
            CellValue::Bool(b) => b.to_string(),
            CellValue::DateTime(dt) => dt.iso.clone().unwrap_or_else(|| dt.serial.to_string()),
            CellValue::Duration(d) => d.iso.clone().unwrap_or_else(|| d.serial.to_string()),
            CellValue::Error(e) => e.to_string(),
            CellValue::Unsupported { display, .. } => display.clone(),
        }
    }

    /// True if the value is `Empty`.
    pub fn is_empty(&self) -> bool {
        matches!(self, CellValue::Empty)
    }

    /// Alias for `display_string` — preferred name per RFC-020.
    #[inline]
    pub fn display_default(&self) -> String {
        self.display_string()
    }
}

// ---------------------------------------------------------------------------
// Display metadata (RFC-020)
// ---------------------------------------------------------------------------

/// Where a display string originated.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize))]
#[non_exhaustive]
pub enum DisplaySource {
    /// Provided directly by the workbook reader.
    ReaderProvided,
    /// Synthesised by `sheets-diff` from the typed value.
    SheetsDiffDefault,
    /// Substituted by the calling application.
    ApplicationProvided,
}

/// A number-format identifier and/or code string captured from the workbook.
///
/// In calamine 0.36 neither field is available from cell data; both are
/// `None` in v2.2. The struct is reserved so RFC-022 can populate it
/// without an API break.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
#[cfg_attr(feature = "serde", derive(Serialize))]
#[non_exhaustive]
pub struct CellNumberFormat {
    /// Excel built-in format ID (e.g. `4` for `#,##0.00`).
    pub id: Option<u32>,
    /// Raw format code string (e.g. `"#,##0.00"`).
    pub code: Option<String>,
}

/// Human-friendly display metadata attached to a cell value (RFC-020).
///
/// `text` is the primary display string. `format` and `source` are optional
/// metadata; consumers may use them for localisation or formatting hints.
#[derive(Clone, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize))]
#[non_exhaustive]
pub struct CellDisplay {
    /// The display string — deterministic and locale-neutral by default.
    pub text: String,
    /// Number-format metadata when available (always `None` in calamine 0.36).
    pub format: Option<CellNumberFormat>,
    pub source: DisplaySource,
}

impl CellDisplay {
    /// Construct a `CellDisplay` from its components.
    pub fn new(text: String, format: Option<CellNumberFormat>, source: DisplaySource) -> Self {
        Self {
            text,
            format,
            source,
        }
    }

    /// Build a default display from a `CellValue`.
    pub fn from_value(value: &CellValue) -> Self {
        Self {
            text: value.display_default(),
            format: None,
            source: DisplaySource::SheetsDiffDefault,
        }
    }
}

/// A full snapshot of one cell: typed value + optional formula + optional display
/// metadata (RFC-020).
///
/// `display` is populated by default using `CellDisplay::from_value`; it can be
/// overridden by the calling application without touching the typed value.
#[derive(Clone, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize))]
#[non_exhaustive]
pub struct CellSnapshot {
    pub value: CellValue,
    pub formula: Option<crate::model::FormulaText>,
    pub display: Option<CellDisplay>,
}

impl CellSnapshot {
    /// Construct a `CellSnapshot` from its components.
    pub fn new(
        value: CellValue,
        formula: Option<FormulaText>,
        display: Option<CellDisplay>,
    ) -> Self {
        Self {
            value,
            formula,
            display,
        }
    }

    /// Return the best available display string: `display.text` when present,
    /// otherwise `value.display_default()`.
    pub fn preferred_display(&self) -> String {
        self.display
            .as_ref()
            .map(|d| d.text.clone())
            .unwrap_or_else(|| self.value.display_default())
    }
}

// ---------------------------------------------------------------------------
// Cell change model (RFC-010 / RFC-033 §5)
// ---------------------------------------------------------------------------

/// Why two `CellValue`s were considered different.
#[non_exhaustive]
#[derive(Clone, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub enum ValueDifferenceKind {
    /// The Rust enum variant changed (e.g. `Integer` → `Number`).
    TypeChanged,
    /// Same type, different content.
    ContentChanged,
    /// Same float type, outside the configured tolerance.
    NumericOutsideTolerance,
    /// Date/time serial or kind changed.
    DateTimeChanged,
    /// `CellError` variant changed.
    ErrorKindChanged,
    /// Compared as display strings (opt-in policy); strings differed.
    DisplayStringChanged,
}

/// A value-layer change at one cell address.
#[derive(Clone, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize))]
#[non_exhaustive]
pub struct ValueChange {
    pub old: CellValue,
    pub new: CellValue,
    pub reason: ValueDifferenceKind,
}

/// A formula's text, with an optional normalised form.
#[derive(Clone, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize))]
#[non_exhaustive]
pub struct FormulaText {
    pub raw: String,
    /// `None` unless the `NormalizedText` formula-compare mode is enabled and
    /// a normaliser is available (RFC-018).
    pub normalized: Option<String>,
}

/// A formula-layer change at one cell address.
///
/// `None` in `old` or `new` means the formula was added or removed.
#[derive(Clone, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize))]
#[non_exhaustive]
pub struct FormulaChange {
    pub old: Option<FormulaText>,
    pub new: Option<FormulaText>,
}

/// Reserved for RFC-022 (style/format diffs).  Always `None` — calamine 0.36
/// does not expose a cell-style API. Set via `FormatCompareMode` (currently
/// only `Ignore` is accepted).
#[derive(Clone, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize))]
#[non_exhaustive]
pub struct FormatChange {
    // Fields added in v2.x once RFC-022 is implemented.
}

/// Derived classification of a `CellDiff` entry.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize))]
#[non_exhaustive]
pub enum CellChangeKind {
    Added,
    Removed,
    Modified,
}

/// A merged per-cell diff entry (RFC-033 §5).
///
/// **One `CellDiff` per logical address.** This is the intended consumer model:
/// a value change and a formula change at the same address are *facets of one
/// change*, carried in the independent `value` and `formula` sub-fields, not
/// two separate entries. The `output::view::CellChangeRow` projection follows
/// the same rule (one row per address, with `formula_changed` / `old_formula` /
/// `new_formula` describing the formula facet). Consumers migrating from a
/// per-facet model should collapse to one row per address rather than preserve
/// the split.
///
/// `change_kind()` is derived from the sub-fields, not stored.
#[non_exhaustive]
#[derive(Clone, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct CellDiff {
    pub address: CellAddress,
    pub value: Option<ValueChange>,
    pub formula: Option<FormulaChange>,
    /// Reserved until RFC-022.
    pub format: Option<FormatChange>,
    pub diagnostics: Vec<Diagnostic>,
}

impl CellDiff {
    /// Derive Added / Removed / Modified from the sub-change fields.
    ///
    /// - **Added**: every present sub-change has an empty/absent `old` side.
    /// - **Removed**: every present sub-change has an empty/absent `new` side.
    /// - **Modified**: otherwise.
    ///
    /// This derivation is **stable API**: the rule above will not change within
    /// a major version, so downstream code may depend on it rather than
    /// re-deriving presence classification from the sub-fields.
    pub fn change_kind(&self) -> CellChangeKind {
        let has_old = self
            .value
            .as_ref()
            .map(|v| !v.old.is_empty())
            .unwrap_or(false)
            || self
                .formula
                .as_ref()
                .map(|f| f.old.is_some())
                .unwrap_or(false);
        let has_new = self
            .value
            .as_ref()
            .map(|v| !v.new.is_empty())
            .unwrap_or(false)
            || self
                .formula
                .as_ref()
                .map(|f| f.new.is_some())
                .unwrap_or(false);
        match (has_old, has_new) {
            (false, true) => CellChangeKind::Added,
            (true, false) => CellChangeKind::Removed,
            _ => CellChangeKind::Modified,
        }
    }
}

// ---------------------------------------------------------------------------
// Diagnostics (RFC-005 / RFC-033 §8)
// ---------------------------------------------------------------------------

/// Severity of a diagnostic entry.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize))]
#[non_exhaustive]
pub enum Severity {
    Info,
    Warning,
    Error,
}

/// Which processing stage emitted a diagnostic.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize))]
#[non_exhaustive]
pub enum DiffStage {
    Open,
    Metadata,
    Match,
    Read,
    Normalize,
    Compare,
    Aggregate,
}

/// Location context attached to a diagnostic.
#[derive(Clone, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize))]
#[non_exhaustive]
pub struct DiagnosticLocation {
    pub stage: DiffStage,
    /// 0-based sheet order (workbook index), if applicable.
    pub sheet_order: Option<usize>,
    pub sheet_name: Option<String>,
    pub address: Option<CellAddress>,
}

/// Structured diagnostic kind.
///
/// `code()` returns a stable string identifier for serde / localisation;
/// it is never renamed within a major version.
#[non_exhaustive]
#[derive(Clone, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub enum DiagnosticKind {
    FormulaUnavailable,
    FormulaCachedValueUnverified,
    AmbiguousSheetMatch {
        candidates: Vec<SheetRef>,
    },
    UnsupportedCellValue {
        detail: String,
    },
    UnsupportedWorkbookFeature {
        feature: String,
    },
    UnsupportedWorkbookMetadata {
        category: String,
    },
    DefinedNameScopeUnknown,
    DateTimeNotNormalized,
    LimitTruncatedCells {
        limit: String,
        observed: u64,
    },
    /// RFC-035 §5.2: the alignment row-product bound (`Limits::max_alignment_product`)
    /// was exceeded, so this sheet fell back to positional comparison. Never
    /// paired with an error — alignment degrades, it does not fail.
    AlignmentBoundExceeded {
        limit: u64,
        observed: u64,
    },
    /// Two or more rows share the same alignment key. Replaces the previous
    /// (incorrect) reuse of `UnsupportedCellValue` for this condition — no
    /// cell value failed to normalise here.
    DuplicateAlignmentKey {
        old_count: usize,
        new_count: usize,
    },
}

impl DiagnosticKind {
    /// Stable code string for this diagnostic kind.
    ///
    /// **These strings are the stable programmatic surface for diagnostics.**
    /// Match on `code()` rather than on the `#[non_exhaustive]` enum variants:
    /// new variants may be added in a minor release (which would break an
    /// exhaustive `match` on the enum), but an existing code string is never
    /// renamed within a major version. Codes also appear verbatim in serialised
    /// JSON.
    ///
    /// The complete set of codes in this major version:
    ///
    /// | Code | Meaning |
    /// |---|---|
    /// | `formula_unavailable` | A cell's formula text could not be read |
    /// | `formula_cached_value_unverified` | A formula's cached value could not be verified |
    /// | `ambiguous_sheet_match` | Sheet rename detection found more than one candidate |
    /// | `unsupported_cell_value` | A cell value could not be normalised to a `CellValue` |
    /// | `unsupported_workbook_feature` | A non-cell object/sheet type is present but not compared |
    /// | `unsupported_workbook_metadata` | A defined-name / visibility / metadata change was detected |
    /// | `defined_name_scope_unknown` | Defined-name scope is unavailable from the reader |
    /// | `datetime_not_normalized` | A date/time value could not be normalised to ISO form |
    /// | `limit_truncated_cells` | A configured cell limit truncated the comparison |
    /// | `alignment_bound_exceeded` | The alignment row-product bound was exceeded; fell back to positional |
    /// | `duplicate_alignment_key` | Two or more rows shared the same alignment key |
    ///
    /// New codes added in later minor versions will extend this table; existing
    /// rows are stable.
    pub fn code(&self) -> &'static str {
        match self {
            DiagnosticKind::FormulaUnavailable => "formula_unavailable",
            DiagnosticKind::FormulaCachedValueUnverified => "formula_cached_value_unverified",
            DiagnosticKind::AmbiguousSheetMatch { .. } => "ambiguous_sheet_match",
            DiagnosticKind::UnsupportedCellValue { .. } => "unsupported_cell_value",
            DiagnosticKind::UnsupportedWorkbookFeature { .. } => "unsupported_workbook_feature",
            DiagnosticKind::UnsupportedWorkbookMetadata { .. } => "unsupported_workbook_metadata",
            DiagnosticKind::DefinedNameScopeUnknown => "defined_name_scope_unknown",
            DiagnosticKind::DateTimeNotNormalized => "datetime_not_normalized",
            DiagnosticKind::LimitTruncatedCells { .. } => "limit_truncated_cells",
            DiagnosticKind::AlignmentBoundExceeded { .. } => "alignment_bound_exceeded",
            DiagnosticKind::DuplicateAlignmentKey { .. } => "duplicate_alignment_key",
        }
    }
}

/// A single structured diagnostic entry.
#[derive(Clone, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize))]
#[non_exhaustive]
pub struct Diagnostic {
    pub severity: Severity,
    pub kind: DiagnosticKind,
    pub location: DiagnosticLocation,
    /// Human-readable message — for display only, not for programmatic matching.
    pub message: String,
}

// ---------------------------------------------------------------------------
// Summary types
// ---------------------------------------------------------------------------

/// Per-sheet summary counts.
#[derive(Clone, Default, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize))]
#[non_exhaustive]
pub struct SheetSummary {
    pub cells_changed: usize,
    pub values_changed: usize,
    pub formulas_changed: usize,
}

/// Diagnostic counts rolled up at any level.
#[derive(Clone, Default, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize))]
#[non_exhaustive]
pub struct DiagnosticSummary {
    pub errors: usize,
    pub warnings: usize,
    pub info: usize,
}

/// Top-level workbook diff summary.
#[derive(Clone, Default, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize))]
#[non_exhaustive]
pub struct DiffSummary {
    pub sheets_added: usize,
    pub sheets_removed: usize,
    pub sheets_renamed: usize,
    pub sheets_moved: usize,
    pub sheets_changed: usize,
    pub cells_changed: usize,
    pub values_changed: usize,
    pub formulas_changed: usize,
    pub diagnostics: DiagnosticSummary,
}

/// Internal processing metrics (RFC-024, RFC-027).
///
/// Useful for benchmarking, performance analysis, and debugging.
/// Always populated; fields are cumulative across the whole comparison.
#[derive(Clone, Default, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize))]
#[non_exhaustive]
pub struct DiffMetrics {
    pub sheets_read: u32,
    pub cells_read: u64,
    pub cells_compared: u64,
    pub diffs_emitted: u64,
    pub diagnostics_emitted: u64,
}

// ---------------------------------------------------------------------------
// SheetDiff
// ---------------------------------------------------------------------------

/// Summary of row-alignment decisions for a sheet pair (RFC-011).
///
/// `None` on `SheetDiff.alignment_summary` when mode is `Positional`.
#[non_exhaustive]
#[derive(Clone, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct AlignmentSummary {
    pub inserted_rows: usize,
    pub removed_rows: usize,
    pub matched_rows: usize,
    pub confidence: MatchConfidence,
}

/// The diff result for one logical sheet pair.
#[non_exhaustive]
#[derive(Clone, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct SheetDiff {
    /// The sheet on the old side (`None` for Added sheets).
    pub old_sheet: Option<SheetRef>,
    /// The sheet on the new side (`None` for Removed sheets).
    pub new_sheet: Option<SheetRef>,
    pub change: SheetChange,
    /// Cell diffs sorted by `(row, col)`.
    pub cell_diffs: Vec<CellDiff>,
    pub compared_range: ComparedRange,
    /// Reserved until RFC-011.
    pub alignment_summary: Option<AlignmentSummary>,
    pub diagnostics: Vec<Diagnostic>,
    pub summary: SheetSummary,
}

// ---------------------------------------------------------------------------
// Workbook-level change placeholders (RFC-021/023, reserved in v2.0)
// ---------------------------------------------------------------------------

/// Reserved for RFC-021 (workbook metadata diffs).  Always empty in v2.0.
#[non_exhaustive]
#[derive(Clone, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct WorkbookChange {
    // Populated by RFC-021 implementation.
}

/// Reserved for RFC-023 (non-cell object diffs).  Always empty in v2.0.
#[non_exhaustive]
#[derive(Clone, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct WorkbookObjectChange {
    // Populated by RFC-023 implementation.
}

// ---------------------------------------------------------------------------
// Top-level result (RFC-033 §12)
// ---------------------------------------------------------------------------

/// The complete diff result for a workbook pair.
///
/// `workbook_changes` and `object_changes` are always empty — RFC-021/023
/// surface their findings through `diagnostics` in v2.2, and structured
/// variants await a future release. The struct is `#[non_exhaustive]` so
/// they can be populated additively without a breaking change.
///
/// # Extracting a lightweight summary
///
/// `summary` ([`DiffSummary`]), `metrics` ([`DiffMetrics`]), and each sheet's
/// `change` ([`SheetChange`]) are all cheap, small, owned values. Memory-conscious
/// consumers that only need counts and the sheet-change list can clone those out
/// and drop the whole `WorkbookDiff` — including the potentially large
/// `sheets[..].cell_diffs` vectors — at their adapter boundary:
///
/// ```no_run
/// # use sheets_diff::compare_paths;
/// let diff = compare_paths("a.xlsx", "b.xlsx")?;
/// let summary = diff.summary.clone();        // cheap
/// let metrics = diff.metrics.clone();        // cheap
/// let sheet_changes: Vec<_> =
///     diff.sheets.iter().map(|s| s.change.clone()).collect();
/// drop(diff);                                 // releases all cell_diffs
/// # Ok::<(), sheets_diff::SheetsDiffError>(())
/// ```
#[non_exhaustive]
#[derive(Clone, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct WorkbookDiff {
    pub old: WorkbookSideInfo,
    pub new: WorkbookSideInfo,
    /// Sheet diffs in old-workbook sheet order (then new-workbook order for
    /// added sheets).
    pub sheets: Vec<SheetDiff>,
    /// Always empty in v2.2; reserved for future structured workbook-level changes.
    pub workbook_changes: Vec<WorkbookChange>,
    /// Always empty in v2.2; reserved for future structured object-level changes.
    pub object_changes: Vec<WorkbookObjectChange>,
    pub diagnostics: Vec<Diagnostic>,
    pub summary: DiffSummary,
    /// Processing metrics for benchmarking and performance analysis (RFC-024/027).
    pub metrics: DiffMetrics,
}

// ---------------------------------------------------------------------------
// Summary derivation helpers
// ---------------------------------------------------------------------------

impl WorkbookDiff {
    pub(crate) fn derive_summary(sheets: &[SheetDiff], diagnostics: &[Diagnostic]) -> DiffSummary {
        let mut s = DiffSummary::default();
        for sd in sheets {
            match sd.change {
                SheetChange::Added => s.sheets_added += 1,
                SheetChange::Removed => s.sheets_removed += 1,
                SheetChange::Renamed { .. } => {
                    s.sheets_renamed += 1;
                    if !sd.cell_diffs.is_empty() {
                        s.sheets_changed += 1;
                    }
                }
                SheetChange::RenamedAndMoved { .. } => {
                    s.sheets_renamed += 1;
                    s.sheets_moved += 1;
                    if !sd.cell_diffs.is_empty() {
                        s.sheets_changed += 1;
                    }
                }
                SheetChange::Moved => {
                    s.sheets_moved += 1;
                    if !sd.cell_diffs.is_empty() {
                        s.sheets_changed += 1;
                    }
                }
                SheetChange::Modified => s.sheets_changed += 1,
                SheetChange::Unchanged => {}
            }
            s.cells_changed += sd.summary.cells_changed;
            s.values_changed += sd.summary.values_changed;
            s.formulas_changed += sd.summary.formulas_changed;
        }
        for d in diagnostics {
            match d.severity {
                Severity::Error => s.diagnostics.errors += 1,
                Severity::Warning => s.diagnostics.warnings += 1,
                Severity::Info => s.diagnostics.info += 1,
            }
        }
        s
    }
}
