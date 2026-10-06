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

/// What kind of input source a workbook came from: one value per way a workbook can be
/// opened. A source that cannot be classified does not exist — the crate builds every
/// [`SourceDescription`] itself, at one of the three entry points — and a future kind of
/// source is added as its own variant (the enum is `#[non_exhaustive]`), not as a fallback.
#[derive(Clone, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize))]
#[non_exhaustive]
pub enum SourceKind {
    /// A file on disk, opened by `compare_paths` / `compare_paths_with_options`.
    Path,
    /// Bytes the caller already holds, given to `compare_bytes` / `compare_bytes_with_options`.
    Bytes,
    /// Any `Read + Seek`, given to `compare_readers` / `compare_readers_with_options`.
    Reader,
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
///
/// **`index` identifies a sheet within its own workbook; `name` does not.** It is assigned once,
/// by position, when the workbook is opened, so it is unique among that workbook's sheets — while
/// two sheets *can* carry the same `name`. Excel's own UI will not write duplicate names, but a
/// file this crate did not produce is under no obligation to avoid them, and comparing `name`
/// where identity was meant is the defect f133 and f134 fixed: it made one sheet claim another's
/// match, and made a third disappear from the result entirely. **Compare `index` to ask "is this
/// the same sheet?"; compare `name` only to ask "do these sheets share a name?"**
///
/// Indices are per workbook, so an old-side `index` and a new-side `index` are only comparable as
/// positions — never as identity across the two sides.
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

/// How much a pairing can be trusted. Shared by two subjects, and each variant says what it means for each.
///
/// - **Sheet matching** (RFC-009) sets `confidence` on a renamed sheet pair, and only `Medium` and `Low`.
/// - **Row alignment** (RFC-011) sets `AlignmentSummary::confidence` for a sheet's rows. It is not derived from
///   the row counts beside it: read `AlignmentSummary::reasons` for why it is what it is.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize))]
#[non_exhaustive]
pub enum MatchConfidence {
    /// Row alignment: every row on both sides was paired by identity, with no ambiguity and no row placed by
    /// content. Sheet matching does not produce `Exact`.
    Exact,
    /// Row alignment: most rows matched, the unmatched ones are genuine insertions and removals, and no reason
    /// applies. Sheet matching does not produce `High`.
    High,
    /// Row alignment: some rows are unmatched, or a reason caps the value (a row placed by content, or a pairing
    /// among identical keys or signatures). Sheet matching: a rename paired on positive but weaker evidence.
    Medium,
    /// Sheet matching: a rename with no positive link between the two sheets, the weakest kind of rename. Row
    /// alignment never produces `Low`: no row-alignment rule yields it, so it is unreachable there.
    Low,
}

/// Why two sheets with **different names** were paired as a rename.
///
/// The matcher considers exactly three things: sheet names, tab positions (indices),
/// and which sheets are left over once the names that agree have been paired. **It
/// never inspects cell content**, so no value here says anything about content. A
/// pair whose names agree is not a rename and carries no reason.
///
/// The variants differ in how much the pairing is worth trusting: see
/// [`MatchConfidence`], which is set alongside.
#[non_exhaustive]
#[derive(Clone, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub enum SheetMatchReason {
    /// The two sheets sit at the same tab position, and the matcher paired them on
    /// that. Under the default sheet-matching mode they were also the only unmatched
    /// sheet on each side; under `ExactNameThenIndex` the position was the only
    /// criterion. No cell content was compared.
    SameIndex,
    /// Neither the names nor the tab positions agree: the pair was formed because it
    /// was the only unmatched sheet left on each side, by elimination. Nothing
    /// positive links the two sheets, and no cell content was compared — treat it as
    /// the weakest kind of rename.
    SoleRemainingPair,
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
///
/// `has_serial` distinguishes a genuine Excel serial (from `Data::DateTime`)
/// from the `0.0` placeholder used when calamine gives only an ISO string
/// (`Data::DateTimeIso`) and no numeric serial exists at all. Comparison
/// (RFC-019 / D-01) must not treat the placeholder as a real serial — a
/// legitimate date can itself serialise to `0.0`, so the placeholder is not
/// otherwise distinguishable from a real one.
#[derive(Clone, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize))]
#[non_exhaustive]
pub struct CellDateTime {
    pub serial: f64,
    pub is_1904: bool,
    pub kind: DateTimeKind,
    pub iso: Option<String>,
    pub has_serial: bool,
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
    /// Cannot occur: the conversion from calamine's `CellErrorType` is an exhaustive match
    /// over its eight kinds, each of which has its own variant above, and nothing else in
    /// this crate constructs it. A match arm on this variant is unreachable today; it is
    /// retained as a forward-compatible catch-all for an Excel error string this crate does
    /// not yet recognise, not as a live case.
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
    /// Cannot occur through any `.xlsx` input this crate accepts: calamine's
    /// `Xlsx` reader routes every numeric cell through `Data::Float`, never
    /// `Data::Int`. A match arm on this variant is unreachable today; it is
    /// retained against future input-format support, not as a live case.
    Integer(i64),
    Number(f64),
    Bool(bool),
    DateTime(CellDateTime),
    /// Cannot occur through any `.xlsx` input this crate accepts: the only
    /// calamine source this maps from, `Data::DurationIso`, is emitted by
    /// calamine's ODS reader only, and this crate opens workbooks exclusively
    /// via `calamine::Xlsx` (`open.rs`). A match arm on this variant is
    /// unreachable today; it is retained against future input-format
    /// support, not as a live case.
    Duration(CellDuration),
    Error(CellError),
    /// Cannot occur: nothing in this crate constructs `Unsupported`. A match
    /// arm on this variant is unreachable today; it is retained as a
    /// forward-compatible catch-all for a future cell shape with no typed
    /// representation here, not as a live case.
    Unsupported {
        display: String,
        reason: String,
    },
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
    /// Display text taken from the workbook reader.
    ///
    /// **This crate never produces this value** — it only ever sets [`SheetsDiffDefault`]
    /// — but it is not unreachable: [`CellDisplay::new`] is public and its `source` is a
    /// public field, so a caller who builds a `CellDisplay` from text a reader gave them
    /// may use it. It is vocabulary for that caller.
    ///
    /// [`SheetsDiffDefault`]: DisplaySource::SheetsDiffDefault
    ReaderProvided,
    /// Synthesised by this crate from the typed value, by [`CellDisplay::from_value`].
    /// The only value this crate produces.
    SheetsDiffDefault,
    /// Display text substituted by the calling application.
    ///
    /// **This crate never produces this value**, but a caller may: build a `CellDisplay`
    /// with [`CellDisplay::new`] and set this as its `source` to mark text the application
    /// chose. It is vocabulary for that caller, not an outcome the engine can reach.
    ApplicationProvided,
}

/// A number-format identifier and/or code string captured from the workbook.
///
/// In calamine 0.36 neither field is available from cell data; both are
/// always `None`. The struct is reserved so RFC-022 can populate it
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
    /// Always `None` today: this crate has no formula normaliser. Reserved for one
    /// (RFC-018 specifies it); the struct is `#[non_exhaustive]`, so populating this
    /// later is not a break.
    pub normalized: Option<String>,
}

/// Why a [`FormulaChange`]'s two texts differ.
///
/// Excel rewrites the row numbers inside a formula when rows move, so under `RowKey` or `RowSignature` a formula
/// that was never edited can still differ as text: `=C5*2` is `=C6*2` after a row is inserted above it. This says
/// whether mapping the old formula's references through the sheet's alignment explains the difference.
///
/// **It annotates and never suppresses.** Every change is reported whatever this says. A caller who wants the
/// cascade gone filters **out** the entries that are [`ExplainedByRowMapping`](Self::ExplainedByRowMapping).
///
/// `#[non_exhaustive]`: the domain is expected to grow (a reference that maps to `#REF!`, a cross-sheet mapping
/// that was unavailable), per RFC-031 §6.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize))]
#[non_exhaustive]
pub enum FormulaDifference {
    /// Mapping the old formula's row references through this sheet's alignment makes the two texts equal. The
    /// formula was not edited; its references moved with the row.
    ExplainedByRowMapping,
    /// Mapping did not explain the difference. Either the old formula was mapped and still differs from the new
    /// one, so it differs for some other reason, which may include an edit; or there was no formula on one side,
    /// so there was nothing to map. The `old` and `new` options of [`FormulaChange`] say which: a formula added or
    /// removed lands here, and is not a variant of its own, because it would restate what `old` and `new`
    /// already carry.
    NotExplainedByRowMapping,
    /// No mapping was attempted, because no row moved to explain anything: the sheet was compared positionally, or
    /// this cell's [`RowPlacement`] is not `PairedByAlignment`.
    NoRowMovement,
    /// Mapping was attempted and declined: part of the formula was not understood, or a reference pointed at a
    /// row with no counterpart, or the sheet's rows were reordered and the formula holds a range. **Nothing is
    /// claimed either way**: this is not "the formula is unchanged". See the CHANGELOG for the classes declined.
    NotDetermined,
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
    /// Why the two texts differ. Always present. See [`FormulaDifference`].
    pub difference: FormulaDifference,
}

/// Reserved for RFC-022 (style/format diffs).  Always `None` — calamine 0.36
/// does not expose a cell-style API. There is no option that turns format comparison
/// on: it returns with RFC-022, as an added option (additive), together with the fields
/// this struct will gain.
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

/// Which row of each sheet a [`CellDiff`] belongs to.
///
/// [`CellDiff::address`] carries one row number, and which sheet that number belongs to depends on
/// how the row was aligned. Read the row numbers from here, not from `address`, whenever a sheet's
/// own numbering matters — for instance when showing a user an address in a file they have open.
///
/// Exhaustive: the domain is closed at four. A fifth variant would need a new alignment concept.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub enum RowPlacement {
    /// Alignment paired this row across the sheets. `address.row` is `old_row`; the same row is
    /// numbered `new_row` in the new sheet. The two numbers are often unequal, and may be equal.
    PairedByAlignment { old_row: u32, new_row: u32 },
    /// Alignment found no counterpart in the new sheet. `address.row` is `old_row`; there is no
    /// new-sheet row number for this row.
    UnpairedInOldSheet { old_row: u32 },
    /// Alignment found no counterpart in the old sheet. `address.row` is `new_row`; there is no
    /// old-sheet row number for this row.
    UnpairedInNewSheet { new_row: u32 },
    /// No alignment ran: row N of the old sheet was compared with row N of the new sheet, so the
    /// one number is correct in both. `address.row` is `row`.
    ComparedPositionally { row: u32 },
}

/// A merged per-cell diff entry (RFC-033 §5).
///
/// **Under `Positional` alignment, one `CellDiff` per address. Under `RowKey` or
/// `RowSignature`, one address can carry more than one.**
///
/// Within a single `CellDiff`, a value change and a formula change at one address
/// are facets of one change, carried in the independent `value` and `formula`
/// sub-fields. That holds in every alignment mode.
///
/// Under `RowKey` or `RowSignature`, two `CellDiff`s can carry the same address
/// and still be distinct changes. Each is numbered in the row space of the side it
/// describes, and the two sides' numbers are independent: a row paired across the
/// sheets is numbered in the old sheet, and a row inserted into the new sheet is
/// numbered in the new sheet, so one number can name two different rows.
/// **Collapsing by address merges those changes and loses one of them.** Keep the
/// `cell_diffs` sequence as it arrives.
///
/// **A change is identified by its address together with its
/// [`row_placement`](Self::row_placement).** Under `Positional` the address alone is enough,
/// because each address then carries one entry; under an aligned mode the address alone does not
/// identify a change, and a map keyed by it loses entries.
///
/// `change_kind()` is derived from the sub-fields, not stored.
#[non_exhaustive]
#[derive(Clone, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct CellDiff {
    pub address: CellAddress,
    /// Which row of each sheet this change belongs to. See [`RowPlacement`].
    pub row_placement: RowPlacement,
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

/// Severity of a diagnostic entry. Ordered: `Info < Warning`.
///
/// There is no `Error`. This crate's model is two-tier: a condition that stops a comparison is a
/// [`SheetsDiffError`](crate::SheetsDiffError) and produces no result; one that does not is a
/// `Diagnostic` and rides along with a successful result. A diagnostic that is fatal has no place in that
/// design, so no severity for it exists — the most severe recoverable condition is a `Warning` ("the diff
/// you are reading may be wrong", as `DuplicateAlignmentKey` says). The enum is `#[non_exhaustive]`, so a
/// further level could be added later without a break.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize))]
#[non_exhaustive]
pub enum Severity {
    /// Something a reader may want to know; the comparison is not in doubt.
    Info,
    /// The comparison completed, but its result may be incomplete or wrong in a way the caller should hear about.
    Warning,
}

/// Which processing stage emitted a diagnostic.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize))]
#[non_exhaustive]
pub enum DiffStage {
    /// Cannot occur: no diagnostic is attributed to this stage today — a workbook that
    /// cannot be opened fails the comparison with an `Err`, it does not warn. A match arm
    /// on this variant is unreachable today; it is retained as the name of a pipeline
    /// stage that exists, not as a live case.
    Open,
    Metadata,
    Match,
    Read,
    /// Cannot occur: no diagnostic is attributed to this stage today — normalising a cell
    /// value emits none. A match arm on this variant is unreachable today; it is retained
    /// as the name of a pipeline stage that exists, not as a live case.
    Normalize,
    Compare,
    /// Cannot occur: no diagnostic is attributed to this stage today — the summary counts
    /// are derived from what the other stages collected and emit nothing themselves. A
    /// match arm on this variant is unreachable today; it is retained as the name of a
    /// pipeline stage that exists, not as a live case.
    Aggregate,
}

/// Where a diagnostic came from.
///
/// **`sheet_order` and `sheet_name` are set together or not at all.** A diagnostic that concerns
/// a particular sheet names it — both fields — as that sheet is in the workbook the diagnostic is
/// about; for a diagnostic about a matched *pair* of sheets that is the **new** workbook's sheet
/// (or the old one's when the sheet exists only there), the label the text renderer uses. A
/// diagnostic that is not about a particular sheet — a defined-name change, the blanket coverage
/// note, an ambiguous rename that concerns several candidates — leaves **both `None`, and that
/// means "not about a sheet", not "not recorded"**. It is never `Some` for one and `None` for the
/// other.
///
/// The same information is available from where a diagnostic sits: one in
/// [`SheetDiff::diagnostics`] is about that sheet, one in [`WorkbookDiff::diagnostics`] is about the
/// workbook — except that a workbook-level diagnostic may still name the sheet it is about (an
/// unsupported chart sheet, a changed visibility), which is why the location carries it too.
#[derive(Clone, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize))]
#[non_exhaustive]
pub struct DiagnosticLocation {
    pub stage: DiffStage,
    /// 0-based position of the sheet in its workbook (its [`SheetRef::index`]); `None` only when
    /// the diagnostic is not about a particular sheet. Set exactly when `sheet_name` is.
    pub sheet_order: Option<usize>,
    /// Name of the sheet the diagnostic is about; `None` only when it is not about a particular
    /// sheet. Set exactly when `sheet_order` is.
    pub sheet_name: Option<String>,
    /// The cell, when the diagnostic is about one.
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
    AmbiguousSheetMatch {
        candidates: Vec<SheetRef>,
    },
    UnsupportedWorkbookFeature {
        feature: String,
    },
    UnsupportedWorkbookMetadata {
        category: String,
    },
    DefinedNameScopeUnknown,
    /// RFC-035 §5.2: the alignment row-product bound (`Limits::max_alignment_product`)
    /// was exceeded, so this sheet fell back to positional comparison. Never
    /// paired with an error — alignment degrades, it does not fail.
    AlignmentBoundExceeded {
        limit: u64,
        observed: u64,
    },
    /// Two or more rows share the same alignment key.
    DuplicateAlignmentKey {
        old_count: usize,
        new_count: usize,
    },
    /// Under `AlignmentMode::RowKey`, rows that have no cell in any key column cannot be matched by
    /// key. They are not dropped: rows with the same values in the same columns on both sides are paired
    /// with one another, and the rest are reported as removed (old side) or inserted (new side), so a
    /// changed row reaches the comparison as a whole-row change. The sheet's alignment confidence is at
    /// most `Medium`. The counts are of keyless rows per side, before pairing.
    MissingAlignmentKey {
        old_count: usize,
        new_count: usize,
    },
    /// Under `AlignmentMode::RowSignature { sample_columns: Some(cols) }`, rows that have no cell in
    /// any sampled column produce no signature and cannot be matched by signature. They are not
    /// dropped: rows with the same values in the same columns on both sides are paired with one
    /// another, and the rest are reported as removed (old side) or inserted (new side), so a changed
    /// row reaches the comparison as a whole-row change, the same rescue `MissingAlignmentKey`
    /// describes for `RowKey`. The counts are of unsampled rows per side, before pairing. Never
    /// raised with `sample_columns: None`: every cell then contributes to its row's signature, so no
    /// row can be excluded from it.
    MissingRowSignature {
        old_count: usize,
        new_count: usize,
    },
    /// Under `AlignmentMode::RowSignature`, a row signature repeats on one side. Rows with identical signatures
    /// are paired by position among themselves, so the pairing among them may be arbitrary, and it is an
    /// ambiguity whether or not it changes the diff. The counts are of distinct repeated signatures per side, as
    /// for `DuplicateAlignmentKey`. A signature is a rendering of each cell's value, not the cell, so an identical
    /// signature can hide a formula difference; the reason is written beside the detection in `src/align.rs`.
    /// The same detection feeds `AlignmentSummary::reasons`.
    DuplicateRowSignature {
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
    /// The complete set of codes in this major version, **every one of which the engine
    /// can produce** (`tests/diagnostic_codes.rs` produces each and asserts the produced
    /// set equals this table — a code that nothing can send is not listed):
    ///
    /// | Code | Meaning |
    /// |---|---|
    /// | `formula_unavailable` | A cell's formula text could not be read |
    /// | `ambiguous_sheet_match` | Sheet rename detection found more than one candidate |
    /// | `unsupported_workbook_feature` | A non-cell object/sheet type is present but not compared |
    /// | `unsupported_workbook_metadata` | A defined-name / visibility / metadata change was detected |
    /// | `defined_name_scope_unknown` | Defined-name scope is unavailable from the reader |
    /// | `alignment_bound_exceeded` | The alignment row-product bound was exceeded; fell back to positional |
    /// | `duplicate_alignment_key` | Two or more rows shared the same alignment key |
    /// | `missing_alignment_key` | Rows with no cell in any key column could not be matched by key; reported as removed / inserted |
    ///
    /// New codes added in later minor versions will extend this table; existing
    /// rows are stable.
    pub fn code(&self) -> &'static str {
        match self {
            DiagnosticKind::FormulaUnavailable => "formula_unavailable",
            DiagnosticKind::AmbiguousSheetMatch { .. } => "ambiguous_sheet_match",
            DiagnosticKind::UnsupportedWorkbookFeature { .. } => "unsupported_workbook_feature",
            DiagnosticKind::UnsupportedWorkbookMetadata { .. } => "unsupported_workbook_metadata",
            DiagnosticKind::DefinedNameScopeUnknown => "defined_name_scope_unknown",
            DiagnosticKind::AlignmentBoundExceeded { .. } => "alignment_bound_exceeded",
            DiagnosticKind::DuplicateAlignmentKey { .. } => "duplicate_alignment_key",
            DiagnosticKind::MissingAlignmentKey { .. } => "missing_alignment_key",
            DiagnosticKind::MissingRowSignature { .. } => "missing_row_signature",
            DiagnosticKind::DuplicateRowSignature { .. } => "duplicate_row_signature",
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

/// The diagnostics a result carries, counted by severity, workbook-level and per-sheet together.
///
/// There is no error count, on purpose: a diagnostic is never fatal — a condition that stops a
/// comparison is a [`SheetsDiffError`](crate::SheetsDiffError), not a diagnostic — so an "errors" figure
/// could only ever be zero. (Through 2.6.0 it was, on every run.)
#[derive(Clone, Default, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize))]
#[non_exhaustive]
pub struct DiagnosticSummary {
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
    /// Sheet pairs processed — one per matched, added, or removed sheet.
    pub sheets_read: u32,
    /// The number of **populated cells** the reader retained, summed over every sheet and both
    /// workbooks: a cell with a value counts once, a styled blank cell (a record with no value)
    /// does not, and a repeated address replaces the cell it repeats rather than adding one — the
    /// size of the cell maps, which is what the comparison spends memory on.
    ///
    /// It is **not** the area of the bounding box of those cells, which is what this figure
    /// reported through 2.6.0. On the `sparse_range` corpus fixture (two populated cells, `A1`
    /// and `Z100`, per side) it is **4** — two cells on each of two sides — against
    /// `cells_compared`'s **2**, the two coordinates compared; the old figure was **5200**, the
    /// area of two 100 x 26 boxes. Where every position inside a sheet's box is populated the two
    /// definitions agree: a dense 5 x 4 block on each side reports 40 either way.
    ///
    /// [`Limits::max_cells_read`](crate::Limits::max_cells_read) bounds this same figure. Always
    /// `>= cells_compared`: every coordinate compared came from at least one retained cell, and
    /// each retained cell belongs to at most one coordinate.
    pub cells_read: u64,
    /// Every coordinate compared between the two sides: the union of both
    /// sides' populated cells for each sheet pair, remapped by alignment
    /// when alignment is not `Positional`. Counted once per coordinate
    /// regardless of whether it produced a diff — always `>= diffs_emitted`.
    pub cells_compared: u64,
    /// Cell-level differences returned in the result — the count behind
    /// every `CellDiff` across all sheets. Always `<= cells_compared`.
    pub diffs_emitted: u64,
    /// `Diagnostic` entries attached to the result, workbook-level and
    /// per-sheet combined.
    pub diagnostics_emitted: u64,
}

// ---------------------------------------------------------------------------
// SheetDiff
// ---------------------------------------------------------------------------

/// Why a row alignment reports the `confidence` it does.
///
/// Read this with [`AlignmentSummary::reasons`]. It is not a decomposition of the three counts beside
/// it: `confidence` is not derived from them, and a reason explains a value, it does not reproduce one.
#[non_exhaustive]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub enum ConfidenceReason {
    /// Some rows had no key or no sampled cell, so they were placed by identical content, not by identity.
    /// Not an ambiguity on its own; it caps `confidence` at `Medium`.
    RowsPlacedByContent,
    /// Under `RowKey`, a key repeats on one side, so the matcher paired among identical keys by position.
    /// An ambiguity: the pairing may be arbitrary, and [`AlignmentSummary::is_ambiguous`] is `true`.
    DuplicateKeys,
    /// Under `RowSignature`, a signature repeats on one side, so the matcher paired among rows with
    /// identical sampled content by position. An ambiguity: [`AlignmentSummary::is_ambiguous`] is `true`.
    DuplicateSignatures,
    /// Rows were unmatched and no ambiguity or content placement applies. Neither an ambiguity nor a
    /// content placement: the count alone sets `confidence`.
    TooFewMatched,
}

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
    /// How much this pairing can be trusted. Read [`reasons`](Self::reasons) for why it is what it is.
    ///
    /// `confidence` is **not** derived from the three counts beside it: two sheets with identical counts can
    /// report different values, because a pairing made among identical keys or signatures, or by identical
    /// content, is capped at `Medium` even when every row matched.
    pub confidence: MatchConfidence,
    /// Why `confidence` is what it is. A set: several may apply at once.
    ///
    /// No reason appears more than once. The order is not meaningful: it is the order the matcher found
    /// the reasons in, and a consumer must not depend on it. An empty list means no reason applies, which
    /// is the case for `Exact` and for `High`. Use [`is_ambiguous`](Self::is_ambiguous) to gate a decision, not
    /// a match over this list.
    pub reasons: Vec<ConfidenceReason>,
}

impl AlignmentSummary {
    /// `true` when a pairing in this sheet was made between identical keys or identical signatures, so the
    /// choice among them was positional.
    ///
    /// This is the question a consumer vetoes on. It is answerable on its own, from this method, and not by
    /// inferring it from the absence of another reason.
    pub fn is_ambiguous(&self) -> bool {
        self.reasons.iter().any(|reason| {
            matches!(
                reason,
                ConfidenceReason::DuplicateKeys | ConfidenceReason::DuplicateSignatures
            )
        })
    }
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
    /// How this sheet's rows were paired, populated under every non-`Positional` alignment mode.
    /// `None` under `Positional`, because no alignment runs there, so there are no alignment
    /// decisions to summarise; for the same reason, confidence cannot be compared between a
    /// positional leg and an aligned leg of the same comparison.
    pub alignment_summary: Option<AlignmentSummary>,
    pub diagnostics: Vec<Diagnostic>,
    pub summary: SheetSummary,
}

// ---------------------------------------------------------------------------
// Workbook-level change placeholders (RFC-021/023, reserved)
// ---------------------------------------------------------------------------

/// Reserved for RFC-021 (workbook metadata diffs).  Always empty.
#[non_exhaustive]
#[derive(Clone, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct WorkbookChange {
    // Populated by RFC-021 implementation.
}

/// Reserved for RFC-023 (non-cell object diffs).  Always empty.
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
/// surface their findings through `diagnostics`, and structured variants
/// await a future release. The struct is `#[non_exhaustive]` so
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
    /// Always empty; reserved for future structured workbook-level changes.
    pub workbook_changes: Vec<WorkbookChange>,
    /// Always empty; reserved for future structured object-level changes.
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
        // Every diagnostic in the result: the workbook's own, and each sheet's. This
        // loop used to read only the first, so a diagnostic attached to a sheet —
        // including `AlignmentBoundExceeded` and `DuplicateAlignmentKey`, the
        // warnings that say the comparison may be wrong — never reached the
        // summary, and disagreed with `DiffMetrics::diagnostics_emitted`, which
        // has always summed both.
        let sheet_level = sheets.iter().flat_map(|sd| sd.diagnostics.iter());
        for d in diagnostics.iter().chain(sheet_level) {
            match d.severity {
                Severity::Warning => s.diagnostics.warnings += 1,
                Severity::Info => s.diagnostics.info += 1,
            }
        }
        s
    }
}
