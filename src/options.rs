//! Comparison options, builder, and related policy enums (RFC-006, RFC-033 §11).

use crate::error::SheetsDiffError;

// ---------------------------------------------------------------------------
// Formula comparison (RFC-018)
// ---------------------------------------------------------------------------

/// How formula text is compared when both sides have a formula.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum FormulaCompareMode {
    /// Compare raw formula strings exactly.  Default.
    #[default]
    RawText,
    /// Do not compare formulas at all.
    Ignore,
}

// ---------------------------------------------------------------------------
// Numeric / value comparison (RFC-019)
// ---------------------------------------------------------------------------

/// How two floating-point numbers are compared.
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub enum NumberComparePolicy {
    /// Bit-faithful parsed equality.  Default.
    #[default]
    Exact,
    AbsoluteTolerance(f64),
    RelativeTolerance(f64),
    AbsoluteOrRelative {
        abs: f64,
        rel: f64,
    },
}

/// Whether `Integer` vs `Number` is treated as a type change.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum NumericTypePolicy {
    /// `Integer(1)` and `Number(1.0)` are **different** (TypeChanged).  Default.
    #[default]
    PreserveType,
    /// Compare by mathematical value; `Integer(1)` and `Number(1.0)` are equal.
    CompareMathematicalValue,
}

/// How date/time values are compared.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum DateComparePolicy {
    /// Compare the raw serial and `is_1904` flag.  Default.
    #[default]
    ExactRepresentation,
    /// Attempt to normalise equivalent date-times before comparing.
    NormalizeEquivalentDateTimes,
}

/// How a typed value is compared against a value of a different type.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum TypeMismatchPolicy {
    /// Different types are always `TypeChanged`.  Default.
    #[default]
    Different,
    /// Compare their display strings instead (for human-friendly reports only).
    CompareDisplayString,
}

/// All value-comparison policy fields grouped together.
///
/// **Build it with [`DiffOptions::builder()`] or with `Default` and field assignment — not with a struct
/// literal.** `ValueCompareOptions` is `#[non_exhaustive]`: an option added to it later is **not** a breaking change,
/// and in exchange a struct expression naming it does not compile outside this crate — not even with
/// `..Default::default()`. Reading and assigning its fields works as always:
///
/// ```compile_fail,E0639
/// use sheets_diff::{NumberComparePolicy, ValueCompareOptions};
/// let _ = ValueCompareOptions { number: NumberComparePolicy::Exact, ..Default::default() };
/// ```
#[derive(Clone, Debug, Default)]
#[non_exhaustive]
pub struct ValueCompareOptions {
    pub number: NumberComparePolicy,
    pub numeric_type: NumericTypePolicy,
    pub date: DateComparePolicy,
    pub type_mismatch: TypeMismatchPolicy,
}

// ---------------------------------------------------------------------------
// Format / style comparison (RFC-022)
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Comparison options
// ---------------------------------------------------------------------------

/// All comparison-behaviour options.
///
/// **Build it with [`DiffOptions::builder()`] or with `Default` and field assignment — not with a struct
/// literal.** `ComparisonOptions` is `#[non_exhaustive]`: an option added to it later is **not** a breaking change,
/// and in exchange a struct expression naming it does not compile outside this crate — not even with
/// `..Default::default()`. Reading and assigning its fields works as always:
///
/// ```compile_fail,E0639
/// use sheets_diff::ComparisonOptions;
/// let _ = ComparisonOptions { include_formula_cached_values: false, ..Default::default() };
/// ```
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct ComparisonOptions {
    pub value: ValueCompareOptions,
    pub formula: FormulaCompareMode,
    /// Whether the formula's cached value is compared as a value change.
    pub include_formula_cached_values: bool,
}

impl Default for ComparisonOptions {
    fn default() -> Self {
        Self {
            value: ValueCompareOptions::default(),
            formula: FormulaCompareMode::default(),
            include_formula_cached_values: true,
        }
    }
}

// ---------------------------------------------------------------------------
// Sheet matching (RFC-009)
// ---------------------------------------------------------------------------

/// How sheets are paired between the two workbooks.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum SheetMatchingMode {
    /// Pair only sheets with the same name; others are Added/Removed.
    ExactNameOnly,
    /// Exact name first; then detect a rename when exactly one unmatched old and
    /// one unmatched new sheet remain and confidence is sufficient.  Default.
    #[default]
    ExactNameThenConservativeRename,
    /// Exact name first; then try pairing by sheet index.
    ExactNameThenIndex,
}

/// Row/column alignment mode (RFC-011).
#[derive(Clone, Debug, Default)]
pub enum AlignmentMode {
    /// Positional (row N on old vs row N on new).  Default.
    #[default]
    Positional,
    /// Match rows by the values in the specified key columns (1-based).
    /// Reduces cascades after row insertion/deletion.
    ///
    /// **This mode needs a key.** A row with no cell in any key column cannot be matched by key. It is
    /// not skipped. Rows with the same values in the same columns on both sides are paired with one
    /// another, in row order, and compared like any matched pair; the rest are reported as removed (old
    /// side) or inserted (new side), so a change in such a row is still seen, as a whole-row change. A
    /// `missing_alignment_key` warning gives the count per side, and the sheet's alignment confidence is
    /// at most `Medium`. A key column that no row populates therefore matches nothing and reports every
    /// row that differs. Duplicate keys warn too (`duplicate_alignment_key`).
    RowKey { columns: Vec<u32> },
    /// Match rows by a hash of selected cell values (content similarity).
    /// `sample_columns` limits which columns contribute to the signature;
    /// `None` means all columns.
    RowSignature { sample_columns: Option<Vec<u32>> },
}

/// Options controlling sheet matching and cell alignment.
///
/// **Build it with [`DiffOptions::builder()`] or with `Default` and field assignment — not with a struct
/// literal.** `MatchingOptions` is `#[non_exhaustive]`: an option added to it later is **not** a breaking change,
/// and in exchange a struct expression naming it does not compile outside this crate — not even with
/// `..Default::default()`. Reading and assigning its fields works as always:
///
/// ```compile_fail,E0639
/// use sheets_diff::{AlignmentMode, MatchingOptions};
/// let _ = MatchingOptions { alignment: AlignmentMode::Positional, ..Default::default() };
/// ```
#[derive(Clone, Debug, Default)]
#[non_exhaustive]
pub struct MatchingOptions {
    pub sheet_matching: SheetMatchingMode,
    pub alignment: AlignmentMode,
}

// ---------------------------------------------------------------------------
// Limits (RFC-012 / RFC-033 §10 / RFC-035 §5.1-5.4)
// ---------------------------------------------------------------------------

/// Default bound on the row-alignment `m × n` table (RFC-035 §5.1, §9).
///
/// Chosen from a direct measurement of `Vec<Vec<u32>>` allocation cost at
/// several square sizes (see Handoff 04's review request for the full
/// table): 5,000×5,000 (this bound) measured ~95 MB / ~15 ms; the
/// previous *unbounded* worst case — two sheets each at the old row-count
/// guard's 50,000-row ceiling — measured ~9.5 GB / ~3.3 s just to
/// zero-allocate the table, before any comparison work. Two sheets each up
/// to ~5,000 rows (or any combination whose product stays under this
/// bound) get full alignment; larger degrades to positional with a
/// diagnostic (RFC-035 §5.2) rather than risking the unbounded case.
pub const DEFAULT_MAX_ALIGNMENT_PRODUCT: u64 = 25_000_000;

/// Default bound on input size, checked before any read begins (RFC-035
/// §5.4): 500 MiB. Chosen to be generous enough that no ordinary `.xlsx`
/// workbook — this crate does not compare macros, embedded media, or other
/// content that would make a legitimate file huge — should ever reach it,
/// while still being finite.
pub const DEFAULT_MAX_INPUT_BYTES: u64 = 500 * 1024 * 1024;

/// Resource bounds that protect against pathological workbooks.
///
/// `None` means no limit on that dimension. Per RFC-035 §5.1, the four
/// *linear* fields (`max_sheets`, `max_cells_read`, `max_cells_compared`,
/// `max_diffs_returned`) default to `None` — their cost scales predictably
/// with input size the caller chose to open, so bounding them by default
/// would surprise working callers for no safety gain they could not have
/// anticipated. `max_alignment_product` and `max_input_bytes` default to
/// `Some` instead: their unbounded cost is *superlinear* or is incurred
/// before any comparison logic can observe it, which is exactly the failure
/// class RFC-035 exists to close. See [`Limits::hardened()`] for a preset
/// that bounds every dimension, for callers who do not trust their input.
///
/// **Build it with [`DiffOptions::builder()`] or with `Default` and field assignment — not with a struct
/// literal.** `Limits` is `#[non_exhaustive]`: an option added to it later is **not** a breaking change,
/// and in exchange a struct expression naming it does not compile outside this crate — not even with
/// `..Default::default()`. Reading and assigning its fields works as always:
///
/// ```compile_fail,E0639
/// use sheets_diff::Limits;
/// let _ = Limits { max_sheets: Some(50), ..Limits::default() };
/// ```
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct Limits {
    pub max_sheets: Option<u32>,
    /// Bounds the number of **populated cells** retained, summed over every sheet and both
    /// workbooks — the figure reported as
    /// [`DiffMetrics::cells_read`](crate::DiffMetrics::cells_read) — and so bounds the memory the
    /// cell maps cost. A styled blank cell is not counted. The bound is evaluated before each cell
    /// is retained, so it fires before the memory it limits is spent; the comparison then fails with
    /// [`LimitExceeded`](crate::SheetsDiffError::LimitExceeded) whose `observed` is `max + 1`.
    ///
    /// Through 2.6.0 this counted the **area of the bounding box** of each sheet's populated cells,
    /// which bounded a dense read's allocation until the read was made to stream, and bounded nothing
    /// any cost after that. A workbook with a vast box and few populated cells — a stray cell far from
    /// the data — is therefore no longer refused by this bound, and none that was accepted is newly refused.
    pub max_cells_read: Option<u64>,
    pub max_cells_compared: Option<u64>,
    pub max_diffs_returned: Option<u64>,
    /// Bounds the `m × n` row-alignment table. Exceeding it degrades this
    /// sheet to positional comparison and emits an
    /// [`AlignmentBoundExceeded`](crate::DiagnosticKind::AlignmentBoundExceeded)
    /// diagnostic — it never errors and never aborts (RFC-035 §5.2). `Some`
    /// by default; see [`DEFAULT_MAX_ALIGNMENT_PRODUCT`].
    pub max_alignment_product: Option<u64>,
    /// Bounds the input size, checked *before* the file is read (or the
    /// reader is drained). Exceeding it returns
    /// [`SheetsDiffError::LimitExceeded`] with
    /// [`LimitKind::InputBytes`](crate::LimitKind::InputBytes) — this one
    /// does error, unlike the alignment bound, because there is no
    /// "positional fallback" for an oversized file. `Some` by default; see
    /// [`DEFAULT_MAX_INPUT_BYTES`].
    pub max_input_bytes: Option<u64>,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_sheets: None,
            max_cells_read: None,
            max_cells_compared: None,
            max_diffs_returned: None,
            max_alignment_product: Some(DEFAULT_MAX_ALIGNMENT_PRODUCT),
            max_input_bytes: Some(DEFAULT_MAX_INPUT_BYTES),
        }
    }
}

impl Limits {
    /// A bound on every dimension [`Limits`] defines, for comparing a workbook
    /// from a source you do not trust (RFC-035 §5.3).
    ///
    /// `Limits::default()` deliberately does **not** provide this — its
    /// four linear fields stay unbounded so ordinary large-but-legitimate
    /// workbooks are never surprised. `hardened()` trades that off: a
    /// caller who opts into it accepts that a very large but legitimate
    /// workbook may hit a limit.
    ///
    /// # What it bounds
    ///
    /// All six fields are set, and each is evaluated before the resource it
    /// limits is spent: input size before any byte is read; sheet count before
    /// any sheet is read; the number of populated cells retained, before each
    /// cell is retained; coordinates compared before a sheet's comparison
    /// loop runs; the row-alignment table before it is allocated; and diffs
    /// returned before the diff is recorded.
    ///
    /// # What it does not bound
    ///
    /// **This is not a guarantee that no workbook can demand unbounded time or
    /// memory.** Two known paths lie outside every dimension `Limits` defines:
    ///
    /// - **Decompression within the size bound.** `max_input_bytes` limits the
    ///   *compressed* input. How far a small archive can expand is decided by
    ///   the `zip` crate, and this crate does not cap it.
    /// - **Cell records that carry no value.** Styled blank cells are skipped
    ///   before `max_cells_read` is evaluated, so it does not count them. They
    ///   cost time, not memory, and are limited only by `max_input_bytes` and by
    ///   cancellation, if you supply a [`Cancellation`].
    ///
    /// The threat model (`docs/src/maintainers/threat-model.md`, the sections
    /// *The zip container* and *Sheet reading*) is the authority on both. Its
    /// *XML parsing* section records a third thing no `Limits` field bounds: the
    /// behaviour this crate inherits from `calamine`'s XML parser.
    ///
    /// # What `max_cells_read` counts
    ///
    /// Populated cells, not the area of their bounding box (which it counted through 2.6.0).
    /// Its bound of 5,000,000 is unchanged, so what `hardened()` accepts changes in **one direction
    /// only**: a workbook whose box was larger than 5,000,000 positions but whose populated cells
    /// were fewer — the sparse-box workbook that motivated the streaming read in 2.5.1 — used to be
    /// refused and is now accepted, at a cost proportional to its populated cells (measured in
    /// `tests/streaming_read.rs`). Nothing that passed before is newly refused, because a box always
    /// contains its cells.
    ///
    /// # Correction
    ///
    /// Versions 2.3.0 through 2.5.1 of this documentation described `hardened()`
    /// as a guarantee that no workbook could demand unbounded time or memory.
    /// That was wrong for the two paths above. The values it sets have not
    /// changed.
    ///
    /// # Values
    ///
    /// Chosen to comfortably accommodate an ordinary office workbook while
    /// bounding each dimension above; they are not individually re-measured
    /// beyond the alignment bound already justified above. If a specific
    /// dimension proves too tight in practice, that is a finding to report, not
    /// a default to silently loosen.
    pub fn hardened() -> Self {
        Self {
            max_sheets: Some(256),
            max_cells_read: Some(5_000_000),
            max_cells_compared: Some(5_000_000),
            max_diffs_returned: Some(1_000_000),
            max_alignment_product: Some(DEFAULT_MAX_ALIGNMENT_PRODUCT),
            max_input_bytes: Some(50 * 1024 * 1024),
        }
    }
}

// ---------------------------------------------------------------------------
// Progress and cancellation (RFC-012)
// ---------------------------------------------------------------------------

/// An event emitted during a comparison for progress reporting.
#[derive(Clone, Debug)]
pub enum DiffEvent {
    Started,
    OpeningWorkbook {
        side: crate::model::Side,
    },
    WorkbookOpened {
        side: crate::model::Side,
        sheet_count: usize,
    },
    MatchingSheets,
    SheetStarted {
        index: usize,
        total: usize,
        name: String,
    },
    SheetFinished {
        index: usize,
        changed_cells: usize,
    },
    Finished,
}

/// Trait for receiving progress events.
///
/// A blanket impl covers any `FnMut(DiffEvent) + Send` closure, so callers can
/// pass a bare closure at call sites without boilerplate (RFC-012).
pub trait ProgressSink: Send {
    fn on_event(&mut self, event: DiffEvent);
}

impl<F: FnMut(DiffEvent) + Send> ProgressSink for F {
    fn on_event(&mut self, event: DiffEvent) {
        self(event);
    }
}

/// Trait for cancellation predicates.
///
/// A blanket impl covers any `Fn() -> bool + Send + Sync`, so the common case
/// is a closure. The single most common adapter is an `Arc<AtomicBool>` shared
/// with a GUI "Cancel" button:
///
/// ```
/// use std::sync::Arc;
/// use std::sync::atomic::{AtomicBool, Ordering};
/// use sheets_diff::DiffOptions;
///
/// let cancel_flag = Arc::new(AtomicBool::new(false));
/// let flag = cancel_flag.clone();
/// let opts = DiffOptions::builder()
///     .cancellation(move || flag.load(Ordering::Relaxed))
///     .build()
///     .unwrap();
/// // Setting `cancel_flag` to true from another thread causes the next
/// // cancellation check to abort the diff with `SheetsDiffError::Cancelled`.
/// ```
///
/// # Cancellation latency
///
/// `is_cancelled()` is polled once before each sheet pair, **and** at an
/// interval inside a sheet's own processing, in both phases:
///
/// - **read phase:** every 50,000 streamed **cell records** — each record the
///   sheet reader yields counts, populated or blank, and a sheet's values pass
///   and formula pass share one counter;
/// - **compare phase:** every 50,000 **coordinates compared**.
///
/// **No latency figure is claimed.** The interval was chosen against a 100 ms
/// target, using a rate of about 1.9 µs per cell for a full sheet-pair pass
/// (300,000 cells in about 567 ms). That rate was measured on the *dense* read
/// that the streaming read has since replaced, and the read phase now counts
/// records rather than positions of a dense range. It has **not been
/// re-measured** on the streamed reader, so 100 ms is the target the interval was
/// derived from, not a guarantee about this code. See
/// `docs/src/maintainers/performance.md` for the original measurement and for the
/// measured overhead of this polling, with and without a `Cancellation`
/// configured.
///
/// **This changed in M7 Handoff 03.** Before it, `is_cancelled()` was polled
/// **only** once before each sheet pair — on a workbook with many sheets,
/// cancellation was observed promptly at the next sheet boundary, but on a
/// single sheet (the ordinary shape of a spreadsheet) there was no next
/// checkpoint, so a comparison ran to completion and returned `Ok` no matter
/// when cancellation was requested. That gap is closed: a single-sheet
/// workbook large enough to cross a polling interval is now cancellable
/// mid-sheet, in both phases. Setting a `max_cells_read` / `max_cells_compared`
/// bound remains useful for a hard resource ceiling, but is no longer the
/// only way to get sub-sheet cancellation latency.
pub trait Cancellation: Send + Sync {
    fn is_cancelled(&self) -> bool;
}

impl<F: Fn() -> bool + Send + Sync> Cancellation for F {
    fn is_cancelled(&self) -> bool {
        self()
    }
}

/// Execution-mode configuration.
///
/// Reserved, currently has no effect: `Sequential` is the only variant and
/// the only path the pipeline runs. A parallel mode was removed (RFC-025,
/// roadmap decision D2) because its implementation parallelised the wrong
/// phase; the type is kept so a future, differently-designed re-introduction
/// does not need a public API break. See RFC-025 for the full rationale and
/// the re-introduction gate.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum ExecutionMode {
    /// Single-threaded, deterministic.  Default.
    #[default]
    Sequential,
}

/// Execution, progress, and cancellation options.
///
/// **Build it with [`DiffOptions::builder()`] or with `Default` and field assignment — not with a struct
/// literal.** `ExecutionOptions` is `#[non_exhaustive]`: an option added to it later is **not** a breaking change,
/// and in exchange a struct expression naming it does not compile outside this crate — not even with
/// `..Default::default()`. Reading and assigning its fields works as always:
///
/// ```compile_fail,E0639
/// use sheets_diff::{ExecutionMode, ExecutionOptions};
/// let _ = ExecutionOptions { mode: ExecutionMode::Sequential, ..Default::default() };
/// ```
#[derive(Default)]
#[non_exhaustive]
pub struct ExecutionOptions {
    pub progress: Option<Box<dyn ProgressSink>>,
    pub cancellation: Option<Box<dyn Cancellation>>,
    /// Reserved, currently has no effect — see [`ExecutionMode`] (RFC-025).
    pub mode: ExecutionMode,
}

// ---------------------------------------------------------------------------
// Diagnostic options
// ---------------------------------------------------------------------------

/// **Build it with [`DiffOptions::builder()`] or with `Default` and field assignment — not with a struct
/// literal.** `DiagnosticOptions` is `#[non_exhaustive]`: an option added to it later is **not** a breaking change,
/// and in exchange a struct expression naming it does not compile outside this crate — not even with
/// `..Default::default()`. Reading and assigning its fields works as always:
///
/// ```compile_fail,E0639
/// use sheets_diff::DiagnosticOptions;
/// let _ = DiagnosticOptions { min_severity: None, ..Default::default() };
/// ```
#[derive(Clone, Debug, Default)]
#[non_exhaustive]
pub struct DiagnosticOptions {
    /// The lowest severity to **collect**.
    ///
    /// The field's default is `None`, which collects every diagnostic — `Info`
    /// and above. `Some(min)` drops any diagnostic below `min` before it is kept:
    /// it appears in neither [`WorkbookDiff::diagnostics`](crate::WorkbookDiff)
    /// nor any [`SheetDiff::diagnostics`](crate::SheetDiff), and no renderer is
    /// involved.
    ///
    /// **The counters follow the filter.** [`DiffSummary::diagnostics`](crate::DiffSummary)
    /// and [`DiffMetrics::diagnostics_emitted`](crate::DiffMetrics) count what was
    /// *kept*, not what the engine generated. That is the honest reading of a
    /// collection filter — the caller asked not to collect them — and the opposite
    /// reading ("count everything, hide some") is equally defensible, so it is
    /// stated rather than left to be guessed. The filter changes which diagnostics
    /// are kept; it does not change which the engine produces.
    ///
    /// This is a different thing from a *display* threshold. The text renderers
    /// choose what to print (the unified renderer shows warnings and errors)
    /// from whatever was collected; they cannot show what this dropped.
    ///
    /// Set it with [`DiffOptionsBuilder::min_severity`], or assign the field:
    ///
    /// ```
    /// use sheets_diff::{DiffOptions, Severity};
    ///
    /// let mut opts = DiffOptions::default();
    /// opts.diagnostics.min_severity = Some(Severity::Warning);
    /// ```
    pub min_severity: Option<crate::model::Severity>,
}

// ---------------------------------------------------------------------------
// Output options
// ---------------------------------------------------------------------------

/// Output and presentation options.
///
/// **Build it with [`DiffOptions::builder()`] or with `Default` and field assignment — not with a struct
/// literal.** `OutputOptions` is `#[non_exhaustive]`: an option added to it later is **not** a breaking change,
/// and in exchange a struct expression naming it does not compile outside this crate — not even with
/// `..Default::default()`. Reading and assigning its fields works as always:
///
/// ```compile_fail,E0639
/// use sheets_diff::{ObjectCompareMode, OutputOptions};
/// let _ = OutputOptions { objects: ObjectCompareMode::Ignore, ..Default::default() };
/// ```
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct OutputOptions {
    /// How non-cell workbook objects are handled (RFC-023).
    pub objects: crate::objects::ObjectCompareMode,
}

impl Default for OutputOptions {
    fn default() -> Self {
        Self {
            objects: crate::objects::ObjectCompareMode::WarnIfPresent,
        }
    }
}

// ---------------------------------------------------------------------------
// DiffOptions — grouped tree (RFC-033 §11)
// ---------------------------------------------------------------------------

/// The top-level configuration entry point for a v2 comparison.
///
/// Construct via `DiffOptions::default()` or `DiffOptions::builder()`. Every field
/// is public, so an option can always be set by assigning it.
///
/// **The builder covers every option.** Each of the nineteen leaf options in the tree below
/// has a method of its own on [`DiffOptionsBuilder`], and none is reachable only by a
/// whole-struct setter that would discard its siblings; `tests/builder_coverage.rs` sets
/// each one through the builder, reads it back, and fails if an option is added without one.
/// Direct field assignment also keeps working — every field is `pub`.
///
/// Setters are named for their option. The four options whose type is a `…Policy` are named
/// for that type in snake_case: [`number_compare_policy`](DiffOptionsBuilder::number_compare_policy),
/// [`numeric_type_policy`](DiffOptionsBuilder::numeric_type_policy),
/// [`type_mismatch_policy`](DiffOptionsBuilder::type_mismatch_policy) and
/// [`date_compare_policy`](DiffOptionsBuilder::date_compare_policy).
///
/// **Build it with [`DiffOptions::builder()`] or with `Default` and field assignment — not with a struct
/// literal.** `DiffOptions` is `#[non_exhaustive]`: an option added to it later is **not** a breaking change,
/// and in exchange a struct expression naming it does not compile outside this crate — not even with
/// `..Default::default()`. Reading and assigning its fields works as always:
///
/// ```compile_fail,E0639
/// use sheets_diff::{DiffOptions, Limits};
/// let _ = DiffOptions { limits: Limits::default(), ..Default::default() };
/// ```
#[derive(Default)]
#[non_exhaustive]
pub struct DiffOptions {
    pub comparison: ComparisonOptions,
    pub matching: MatchingOptions,
    pub limits: Limits,
    pub execution: ExecutionOptions,
    pub diagnostics: DiagnosticOptions,
    pub output: OutputOptions,
}

impl DiffOptions {
    pub fn builder() -> DiffOptionsBuilder {
        DiffOptionsBuilder::new()
    }

    /// Validate option combinations before I/O begins.
    ///
    /// **No combination of options is currently invalid**, so this always succeeds. It remains
    /// the one place a future option that *can* be set to something unusable would be checked —
    /// `build()` and every comparison entry point already call it, and
    /// [`SheetsDiffError::InvalidOptions`] is what it would return.
    pub(crate) fn validate(&self) -> Result<(), SheetsDiffError> {
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Builder
// ---------------------------------------------------------------------------

/// Fluent builder for `DiffOptions`.
///
/// Call `.build()` to validate the combination and obtain a `DiffOptions`.
#[derive(Default)]
pub struct DiffOptionsBuilder {
    opts: DiffOptions,
}

impl DiffOptionsBuilder {
    pub fn new() -> Self {
        Self {
            opts: DiffOptions::default(),
        }
    }

    // Comparison

    pub fn formula_compare(mut self, mode: FormulaCompareMode) -> Self {
        self.opts.comparison.formula = mode;
        self
    }

    /// Set the object comparison mode (RFC-023).
    pub fn object_mode(mut self, mode: crate::objects::ObjectCompareMode) -> Self {
        self.opts.output.objects = mode;
        self
    }

    /// Set the execution mode.
    ///
    /// Reserved, currently has no effect — see [`ExecutionMode`] (RFC-025).
    pub fn execution_mode(mut self, mode: ExecutionMode) -> Self {
        self.opts.execution.mode = mode;
        self
    }

    pub fn include_formula_cached_values(mut self, yes: bool) -> Self {
        self.opts.comparison.include_formula_cached_values = yes;
        self
    }

    pub fn numeric_type_policy(mut self, policy: NumericTypePolicy) -> Self {
        self.opts.comparison.value.numeric_type = policy;
        self
    }

    pub fn type_mismatch_policy(mut self, policy: TypeMismatchPolicy) -> Self {
        self.opts.comparison.value.type_mismatch = policy;
        self
    }

    pub fn number_compare_policy(mut self, policy: NumberComparePolicy) -> Self {
        self.opts.comparison.value.number = policy;
        self
    }

    /// Set how date/time values are compared. The default is
    /// [`DateComparePolicy::ExactRepresentation`]; see [`DateComparePolicy`].
    ///
    /// ```
    /// use sheets_diff::{DateComparePolicy, DiffOptions};
    ///
    /// let opts = DiffOptions::builder()
    ///     .date_compare_policy(DateComparePolicy::NormalizeEquivalentDateTimes)
    ///     .build()?;
    /// # let _ = opts;
    /// # Ok::<(), sheets_diff::SheetsDiffError>(())
    /// ```
    pub fn date_compare_policy(mut self, policy: DateComparePolicy) -> Self {
        self.opts.comparison.value.date = policy;
        self
    }

    // Matching

    pub fn sheet_matching(mut self, mode: SheetMatchingMode) -> Self {
        self.opts.matching.sheet_matching = mode;
        self
    }

    /// Set how rows are aligned between the two sides of a matched sheet.
    ///
    /// The default is [`AlignmentMode::Positional`]. The other modes match rows by
    /// content and may raise sheet-level diagnostics; see [`AlignmentMode`].
    /// This sets only the alignment: it leaves [`sheet_matching`](Self::sheet_matching)
    /// alone, in either call order.
    ///
    /// ```
    /// use sheets_diff::DiffOptions;
    /// use sheets_diff::options::AlignmentMode;
    ///
    /// let opts = DiffOptions::builder()
    ///     .alignment(AlignmentMode::RowKey { columns: vec![1] })
    ///     .build()?;
    /// # let _ = opts;
    /// # Ok::<(), sheets_diff::SheetsDiffError>(())
    /// ```
    pub fn alignment(mut self, mode: AlignmentMode) -> Self {
        self.opts.matching.alignment = mode;
        self
    }

    // Limits

    pub fn max_sheets(mut self, n: u32) -> Self {
        self.opts.limits.max_sheets = Some(n);
        self
    }

    /// Bounds the figure reported as [`DiffMetrics::cells_read`](crate::DiffMetrics::cells_read);
    /// `None`, the default, is unbounded. What that figure counts is defined there, once, and
    /// not repeated here. Takes an `Option` so that `None` can be said, as
    /// [`max_alignment_product`](Self::max_alignment_product) does.
    ///
    /// ```
    /// use sheets_diff::DiffOptions;
    ///
    /// let opts = DiffOptions::builder().max_cells_read(Some(1_000_000)).build()?;
    /// # let _ = opts;
    /// # Ok::<(), sheets_diff::SheetsDiffError>(())
    /// ```
    pub fn max_cells_read(mut self, limit: Option<u64>) -> Self {
        self.opts.limits.max_cells_read = limit;
        self
    }

    pub fn max_cells_compared(mut self, n: u64) -> Self {
        self.opts.limits.max_cells_compared = Some(n);
        self
    }

    pub fn max_diffs_returned(mut self, n: u64) -> Self {
        self.opts.limits.max_diffs_returned = Some(n);
        self
    }

    /// Bounds the `m × n` alignment table; `Some` by default
    /// ([`DEFAULT_MAX_ALIGNMENT_PRODUCT`]). Pass `None` to disable the
    /// bound entirely (RFC-035 §5.1 — this is opt-out, not opt-in).
    pub fn max_alignment_product(mut self, limit: Option<u64>) -> Self {
        self.opts.limits.max_alignment_product = limit;
        self
    }

    /// Bounds input size, checked before any read begins; `Some` by
    /// default ([`DEFAULT_MAX_INPUT_BYTES`]). Pass `None` to disable the
    /// bound entirely.
    pub fn max_input_bytes(mut self, limit: Option<u64>) -> Self {
        self.opts.limits.max_input_bytes = limit;
        self
    }

    /// Replace all limits at once, e.g. with [`Limits::hardened()`].
    pub fn limits(mut self, limits: Limits) -> Self {
        self.opts.limits = limits;
        self
    }

    // Diagnostics

    /// Set the lowest severity to **collect**; `None`, the default, collects
    /// every diagnostic.
    ///
    /// This is a collection filter, not a display threshold. What it drops, and
    /// what the counters then report, is defined once, on the field:
    /// [`DiagnosticOptions::min_severity`].
    ///
    /// Takes an `Option` so that `None` can be said, not only omitted.
    ///
    /// ```
    /// use sheets_diff::{DiffOptions, Severity};
    ///
    /// let opts = DiffOptions::builder()
    ///     .min_severity(Some(Severity::Warning))
    ///     .build()?;
    /// # let _ = opts;
    /// # Ok::<(), sheets_diff::SheetsDiffError>(())
    /// ```
    pub fn min_severity(mut self, severity: Option<crate::model::Severity>) -> Self {
        self.opts.diagnostics.min_severity = severity;
        self
    }

    // Execution

    pub fn progress<S: ProgressSink + 'static>(mut self, sink: S) -> Self {
        self.opts.execution.progress = Some(Box::new(sink));
        self
    }

    pub fn cancellation<C: Cancellation + 'static>(mut self, token: C) -> Self {
        self.opts.execution.cancellation = Some(Box::new(token));
        self
    }

    /// Validate and return the built options.
    pub fn build(self) -> Result<DiffOptions, SheetsDiffError> {
        self.opts.validate()?;
        Ok(self.opts)
    }
}
