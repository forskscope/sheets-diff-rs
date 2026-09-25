# RFC-005: Error, Diagnostics, and Warning Model

**Status.** Implemented (2.0.0–2.2.3) — verified 2026-08-16 against the implementation.
**Two gaps that verification missed, corrected by M8 unit 02 (2.6.0):** (1) a
diagnostic attached to a *sheet* reached no output — `DiffSummary::diagnostics`
counted only the workbook-level vector (so it disagreed with
`DiffMetrics::diagnostics_emitted`, which always summed both), and neither text
renderer looked at `SheetDiff::diagnostics`, which hid `AlignmentBoundExceeded`
and `DuplicateAlignmentKey`, the warnings that say a comparison may be wrong. That
held from 2.0.0. The acceptance line below — "Recoverable sheet warnings appear in
`WorkbookDiff`/`SheetDiff` diagnostics" — was true of the model and false of every
output. (2) `DiagnosticOptions::min_severity` was public, documented, and read by
nothing. It is now a *collection* filter, applied once in the engine, and the
summary counters follow what was kept; `--no-warnings` is a separate, *display*
control. See `rfcs/handoffs/m8-the-surface/02-two-inert-options.md`.
**Target:** v2.0.0  
**Created:** 2026-06-11  
**Category:** Error handling  

## 1. Summary

Define structured fatal errors and recoverable diagnostics so the library never prints from core code and never panics for ordinary bad input.

## 2. Motivation

GUI applications need localized, contextual error handling. CLI tools need exit codes and messages. Libraries should not write to stdout/stderr, and ordinary malformed input should not crash a process. v2 needs a first-class diagnostic model.

## 3. Goals

- Provide a non-panicking error type for fatal failures.
- Represent recoverable issues as diagnostics attached to results.
- Support strict and lenient modes where practical.
- Make diagnostics localizable by providing codes and structured fields.
- Ensure core library code has no stdout/stderr writes.

## 4. Non-goals

- Do not implement localization strings in the library.
- Do not swallow fatal errors silently.
- Do not expose raw panic messages as normal diagnostics.

## 5. External design

Proposed fatal error type:

```rust
#[derive(Debug)]
pub enum SheetsDiffError {
    OpenWorkbook { side: Side, source: SourceDescription, message: String },
    ReadWorkbook { side: Side, message: String },
    ReadSheet { side: Side, sheet: SheetRef, message: String },
    UnsupportedWorkbook { side: Side, reason: UnsupportedReason },
    Cancelled,
    LimitExceeded { limit: LimitKind, observed: usize },
    InternalInvariant { message: String },
}
```

Recoverable diagnostics:

```rust
pub struct Diagnostic {
    pub severity: Severity,
    pub code: DiagnosticCode,
    pub location: DiagnosticLocation,
    pub message: String,
}
```

`message` is for convenience; consumers should rely on code/location for localization.

**`DiagnosticLocation` — the rule (M10 unit 04, unreleased).** *A diagnostic that concerns a particular
sheet names it — `sheet_order` and `sheet_name` together, never one without the other — as that sheet is in
the workbook the diagnostic is about (for a matched pair, the new workbook's, else the old one's, the label
the text renderer uses); a diagnostic that is not about a particular sheet leaves both `None`, which means
"not about a sheet", not "nobody set this".* This RFC did not say what the location must contain, and the
implementation filled it where it was convenient: of eleven construction sites two set both fields, one set only
the name, and eight none — including the two sheet-level alignment warnings (`AlignmentBoundExceeded`,
`DuplicateAlignmentKey`), which the engine pushes into a specific `SheetDiff` yet recorded nothing. A JSON consumer
could not tell "not about a sheet" from "not set". Now: the two alignment warnings and the sheet-visibility
diagnostic name their sheet (the last used to set the name without the order); the defined-name diagnostics, the
ambiguous-rename warning and the blanket coverage note deliberately stay `None`. `tests/diagnostic_location.rs` pins
both directions.

## 6. Internal design

Errors should be created at boundary points: source open, workbook read, sheet range read, normalization, cancellation, and bounds checks.

Diagnostics should be aggregated in three layers:

```text
WorkbookDiff.diagnostics
SheetDiff.diagnostics
CellDiff.diagnostics
```

Internal helpers should return `Result<T, SheetsDiffError>` or `Result<T, Diagnostic>` only when the distinction between fatal and recoverable is clear. Avoid `anyhow` in public API.

## 7. Data lifecycle

1. Open failures produce fatal errors.
2. Sheet read failures in strict mode produce fatal errors.
3. Sheet read failures in lenient mode produce diagnostics and partial result when safe.
4. Ambiguous sheet matching produces warning diagnostics.
5. Cancellation and limits produce fatal but expected errors.

## 8. Error, diagnostic, and edge-case behavior

All ordinary user/file problems must be data, not panics. Panics are reserved for programmer bugs and must not be part of normal error handling.

`InternalInvariant` should be rare and should indicate a bug in `sheets-diff`, not user input.

## 9. Testing and acceptance criteria

Acceptance criteria:

- No `println!`, `eprintln!`, or direct logging in library core.
- Bad input tests assert `Err`, not panic.
- Recoverable sheet warnings appear in `WorkbookDiff`/`SheetDiff` diagnostics.
- CLI maps errors to stable exit codes.
- Diagnostic codes are documented.

## 10. Migration and compatibility

v1 consumers using `catch_unwind` can remove it and handle `SheetsDiffError`. Existing stdout scraping should be replaced with diagnostics.

## 11. Open questions

- Should the public error type wrap `calamine::XlsxError` directly or convert to string to avoid dependency leakage?
- Should diagnostics use string codes for schema stability or Rust enums for type safety?
