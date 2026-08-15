# RFC-005: Error, Diagnostics, and Warning Model

**Status.** Implemented (2.0.0–2.2.3) — restored 2026-08-15; not individually re-verified against the implementation.
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
