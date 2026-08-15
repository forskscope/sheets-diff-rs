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
    AbsoluteOrRelative { abs: f64, rel: f64 },
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
/// Default is `Ignore` — calamine 0.35 does not expose a cell-style API, so
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
// Limits (RFC-012 / RFC-033 §10)
// ---------------------------------------------------------------------------

/// Resource bounds that protect against pathological workbooks.
///
/// `None` means no limit on that dimension.
#[derive(Clone, Debug, Default)]
pub struct Limits {
    pub max_sheets: Option<u32>,
    pub max_cells_read: Option<u64>,
    pub max_cells_compared: Option<u64>,
    pub max_diffs_returned: Option<u64>,
}

// ---------------------------------------------------------------------------
// Progress and cancellation (RFC-012)
// ---------------------------------------------------------------------------

/// An event emitted during a comparison for progress reporting.
#[derive(Clone, Debug)]
pub enum DiffEvent {
    Started,
    OpeningWorkbook { side: crate::model::Side },
    WorkbookOpened { side: crate::model::Side, sheet_count: usize },
    MatchingSheets,
    SheetStarted { index: usize, total: usize, name: String },
    SheetFinished { index: usize, changed_cells: usize },
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
/// `is_cancelled()` is polled **once before each sheet pair** is processed.
/// On a workbook with many sheets, cancellation is observed promptly. On a
/// single very large sheet, cancellation is **not** observed mid-sheet in the
/// current implementation — it fires before the next sheet begins. If you need
/// sub-sheet cancellation latency for huge single-sheet workbooks, also set a
/// `max_cells_read` / `max_cells_compared` bound so the diff returns within a
/// predictable amount of work.
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
pub struct ExecutionOptions {
    pub progress: Option<Box<dyn ProgressSink>>,
    pub cancellation: Option<Box<dyn Cancellation>>,
    /// Reserved, currently has no effect — see [`ExecutionMode`] (RFC-025).
    pub mode: ExecutionMode,
}

impl Default for ExecutionOptions {
    fn default() -> Self {
        Self {
            progress: None,
            cancellation: None,
            mode: ExecutionMode::default(),
        }
    }
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
        Self { objects: crate::objects::ObjectCompareMode::WarnIfPresent }
    }
}

// ---------------------------------------------------------------------------
// DiffOptions — grouped tree (RFC-033 §11)
// ---------------------------------------------------------------------------

/// The top-level configuration entry point for a v2 comparison.
///
/// Construct via `DiffOptions::default()` or `DiffOptions::builder()`.
pub struct DiffOptions {
    pub comparison: ComparisonOptions,
    pub matching: MatchingOptions,
    pub limits: Limits,
    pub execution: ExecutionOptions,
    pub diagnostics: DiagnosticOptions,
    pub output: OutputOptions,
}

impl Default for DiffOptions {
    fn default() -> Self {
        Self {
            comparison: ComparisonOptions::default(),
            matching: MatchingOptions::default(),
            limits: Limits::default(),
            execution: ExecutionOptions::default(),
            diagnostics: DiagnosticOptions::default(),
            output: OutputOptions::default(),
        }
    }
}

impl DiffOptions {
    pub fn builder() -> DiffOptionsBuilder {
        DiffOptionsBuilder::new()
    }

    /// Validate option combinations before I/O begins.
    pub(crate) fn validate(&self) -> Result<(), SheetsDiffError> {
        // NormalizedText requires a normaliser; none exists in v2.0.
        if self.comparison.formula == FormulaCompareMode::NormalizedText
            || self.comparison.formula == FormulaCompareMode::RawAndNormalized
        {
            return Err(SheetsDiffError::InvalidOptions {
                detail: "FormulaCompareMode::NormalizedText / RawAndNormalized is not \
                         available in v2.0; no formula normaliser is implemented yet"
                    .into(),
            });
        }
        // Style comparison requires a calamine style reader not yet available.
        if self.comparison.format != FormatCompareMode::Ignore {
            return Err(SheetsDiffError::InvalidOptions {
                detail: "FormatCompareMode other than Ignore is not available in v2; \
                         calamine 0.35 does not expose a cell-style API"
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
        Self { opts: DiffOptions::default() }
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
    pub fn build_with_matching(mut self, matching: MatchingOptions) -> Result<DiffOptions, SheetsDiffError> {
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
