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
    /// Compare normalised formula strings.  Requires a normaliser feature;
    /// returns `InvalidOptions` if selected without one.
    NormalizedText,
    /// Compare both raw and normalised; emits both in `FormulaText`.
    RawAndNormalized,
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
#[derive(Clone, Debug, Default)]
pub struct ValueCompareOptions {
    pub number: NumberComparePolicy,
    pub numeric_type: NumericTypePolicy,
    pub date: DateComparePolicy,
    pub type_mismatch: TypeMismatchPolicy,
}

// ---------------------------------------------------------------------------
// Format / style comparison (RFC-022)
// ---------------------------------------------------------------------------

/// Controls whether cell formatting (number format, font, fill, …) is compared.
///
/// Default is `Ignore` — calamine 0.36 does not expose a cell-style API, so
/// `AllAvailable` emits an `UnsupportedWorkbookFeature` diagnostic at runtime.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum FormatCompareMode {
    /// Ignore all formatting differences (default).
    #[default]
    Ignore,
    /// Compare number-format strings only (future, requires style reader).
    NumberFormatOnly,
    /// Compare all available style fields (future, best-effort).
    AllAvailable,
}

// ---------------------------------------------------------------------------
// Comparison options
// ---------------------------------------------------------------------------

/// All comparison-behaviour options.
#[derive(Clone, Debug)]
pub struct ComparisonOptions {
    pub value: ValueCompareOptions,
    pub formula: FormulaCompareMode,
    /// Whether the formula's cached value is compared as a value change.
    pub include_formula_cached_values: bool,
    /// Cell formatting comparison mode (RFC-022). Default: `Ignore`.
    pub format: FormatCompareMode,
}

impl Default for ComparisonOptions {
    fn default() -> Self {
        Self {
            value: ValueCompareOptions::default(),
            formula: FormulaCompareMode::default(),
            include_formula_cached_values: true,
            format: FormatCompareMode::default(),
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
#[allow(dead_code)]
#[derive(Clone, Debug, Default)]
pub enum AlignmentMode {
    /// Positional (row N on old vs row N on new).  Default.
    #[default]
    Positional,
    /// Match rows by the values in the specified key columns (1-based).
    /// Reduces cascades after row insertion/deletion.
    RowKey { columns: Vec<u32> },
    /// Match rows by a hash of selected cell values (content similarity).
    /// `sample_columns` limits which columns contribute to the signature;
    /// `None` means all columns.
    RowSignature { sample_columns: Option<Vec<u32>> },
    /// Match rows using the first row as a column-header identity.
    #[allow(dead_code)]
    HeaderColumn,
}

/// Options controlling sheet matching and cell alignment.
#[derive(Clone, Debug, Default)]
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
#[derive(Clone, Debug)]
pub struct Limits {
    pub max_sheets: Option<u32>,
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
    /// A conservative bound on every dimension, for comparing a workbook
    /// from a source you do not trust (RFC-035 §5.3).
    ///
    /// `Limits::default()` deliberately does **not** provide this — its
    /// four linear fields stay unbounded so ordinary large-but-legitimate
    /// workbooks are never surprised. `hardened()` trades that off: a
    /// caller who opts into it accepts that a very large but legitimate
    /// workbook may hit a limit, in exchange for a guarantee that no
    /// workbook — hostile or merely huge — can demand unbounded time or
    /// memory. Values are chosen to comfortably accommodate an ordinary
    /// office workbook while capping the worst case; they are not
    /// individually re-measured beyond the alignment bound already
    /// justified above; if a specific dimension proves too tight in
    /// practice, that is a finding to report, not a default to silently
    /// loosen.
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
/// interval inside a sheet's own processing — every 50,000 cells, in both
/// the read phase and the compare phase. On the largest single sheet this
/// crate's own benchmark ladder covers (300,000 cells), that bounds
/// worst-case latency to roughly 100 ms; see `docs/src/maintainers/performance.md`
/// for the measured overhead of this polling, with and without a
/// `Cancellation` configured.
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
#[derive(Default)]
pub struct ExecutionOptions {
    pub progress: Option<Box<dyn ProgressSink>>,
    pub cancellation: Option<Box<dyn Cancellation>>,
    /// Reserved, currently has no effect — see [`ExecutionMode`] (RFC-025).
    pub mode: ExecutionMode,
}

// ---------------------------------------------------------------------------
// Diagnostic options
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Default)]
pub struct DiagnosticOptions {
    /// Minimum severity to collect.  Defaults to `Info` (collect everything).
    pub min_severity: Option<crate::model::Severity>,
}

// ---------------------------------------------------------------------------
// Output options
// ---------------------------------------------------------------------------

/// Output and presentation options.
#[derive(Clone, Debug)]
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
/// Construct via `DiffOptions::default()` or `DiffOptions::builder()`.
#[derive(Default)]
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
    pub(crate) fn validate(&self) -> Result<(), SheetsDiffError> {
        // NormalizedText requires a normaliser; none exists.
        if self.comparison.formula == FormulaCompareMode::NormalizedText
            || self.comparison.formula == FormulaCompareMode::RawAndNormalized
        {
            return Err(SheetsDiffError::InvalidOptions {
                detail: "FormulaCompareMode::NormalizedText / RawAndNormalized is not \
                         available; no formula normaliser is implemented yet"
                    .into(),
            });
        }
        // Style comparison requires a calamine style reader not yet available.
        if self.comparison.format != FormatCompareMode::Ignore {
            return Err(SheetsDiffError::InvalidOptions {
                detail: "FormatCompareMode other than Ignore is not available in v2; \
                         calamine 0.36 does not expose a cell-style API"
                    .into(),
            });
        }
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

    pub fn format_compare(mut self, mode: FormatCompareMode) -> Self {
        self.opts.comparison.format = mode;
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

    pub fn number_compare(mut self, policy: NumberComparePolicy) -> Self {
        self.opts.comparison.value.number = policy;
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

    // Matching

    pub fn sheet_matching(mut self, mode: SheetMatchingMode) -> Self {
        self.opts.matching.sheet_matching = mode;
        self
    }

    // Limits

    pub fn max_sheets(mut self, n: u32) -> Self {
        self.opts.limits.max_sheets = Some(n);
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

    // Execution

    pub fn progress<S: ProgressSink + 'static>(mut self, sink: S) -> Self {
        self.opts.execution.progress = Some(Box::new(sink));
        self
    }

    pub fn cancellation<C: Cancellation + 'static>(mut self, token: C) -> Self {
        self.opts.execution.cancellation = Some(Box::new(token));
        self
    }

    /// Build with a fully specified `MatchingOptions` (convenience for alignment tests).
    pub fn build_with_matching(
        mut self,
        matching: MatchingOptions,
    ) -> Result<DiffOptions, SheetsDiffError> {
        self.opts.matching = matching;
        self.opts.validate()?;
        Ok(self.opts)
    }

    /// Validate and return the built options.
    pub fn build(self) -> Result<DiffOptions, SheetsDiffError> {
        self.opts.validate()?;
        Ok(self.opts)
    }
}
