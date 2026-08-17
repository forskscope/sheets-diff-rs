# RFC-032 — Unsupported, Corrupt, and Encrypted Workbook Handling

**Status.** Implemented (2.0.0–2.4.x) — verified 2026-08-16; the deferral closed 2026-08-17 (M5 Handoff 03). Encrypted-workbook detection (`SheetsDiffError::EncryptedWorkbook` via `from_open_error`) is covered by `tests/encrypted_workbook.rs` and `tests/cli.rs`: both sides yield the variant with the correct `Side`, via `compare_bytes` and `compare_paths`; the rendered message names the condition, with a negative control confirming an ordinary corrupt input does not; and the CLI exits 3. Fixture: `tests/fixtures/corrupt/encrypted.xlsx`, provenance in that directory's `README.md`.
**Target:** v2.0 decision  
**Related:** RFC-004, RFC-005, RFC-016, RFC-028

## 1. Summary

Define how v2 handles unsupported file types, corrupt `.xlsx` packages,
password-protected/encrypted workbooks, locked files, and workbooks with partially
unsupported contents.

## 2. Motivation

The original field report emphasized that GUI applications cannot accept panics
from ordinary bad input. Users will select wrong files, corrupt downloads,
password-protected workbooks, temporary lock files, and unsupported spreadsheet
formats. v2 must turn these into structured errors or diagnostics.

## 3. Goals

- Return structured errors for unreadable or invalid input.
- Distinguish unsupported format from corrupt workbook where possible.
- Distinguish encrypted/password-protected workbooks where possible.
- Support partial diagnostics for unsupported internal workbook features.
- Never print to stdout/stderr or panic for ordinary input failures.

## 4. Non-goals

- Password cracking or decryption.
- Supporting `.xls`, `.ods`, `.csv`, or other formats in v2.0 unless explicitly
  accepted elsewhere.
- Repairing corrupt workbooks.
- Following external links.

## 5. Error model

```rust
pub enum SheetsDiffError {
    Open(OpenWorkbookError),
    Read(ReadWorkbookError),
    UnsupportedFormat(UnsupportedFormatError),
    EncryptedWorkbook(EncryptedWorkbookError),
    InvalidOptions(InvalidOptionsError),
    Cancelled(CancelledError),
    LimitExceeded(LimitExceededError),
}
```

Open/read errors should preserve a source error when available without exposing
unstable dependency types as mandatory public API.

## 6. Unsupported format policy

If the caller uses an `.xlsx` API with non-xlsx input, return
`UnsupportedFormat` or `Open` with a clear kind. Do not infer solely from file
extension; bytes/reader APIs may have no extension.

Use content inspection where the reader provides it.

## 7. Encrypted workbook policy

If encryption/password protection is detected, return:

```rust
SheetsDiffError::EncryptedWorkbook { side, source_name }
```

If the underlying reader reports only a generic parse error, include a message
that the workbook may be encrypted/corrupt only if evidence supports it.

No password parameter is accepted in v2.0. Password support would require a
separate security review.

## 8. Partial unsupported contents

A workbook may be valid but contain unsupported features. This should usually be
a diagnostic, not an error:

```rust
DiagnosticKind::UnsupportedWorkbookFeature {
    feature: WorkbookFeatureKind,
    location: Option<DiagnosticLocation>,
    compared_cells_unaffected: Option<bool>,
}
```

For example, unsupported charts do not prevent cell comparison. Unsupported cell
value types might affect comparison and should have higher severity.

## 9. File locking and permissions

Path APIs should preserve I/O errors enough for GUI callers to show useful
messages: not found, permission denied, directory instead of file, locked or
busy where the OS reports it.

## 10. Acceptance criteria

- Missing file returns structured open error.
- Random bytes through bytes API return structured error.
- Valid ZIP but not XLSX returns unsupported/corrupt error, not panic.
- Password-protected workbook fixture returns encrypted/unsupported structured
  error if detectable.
- Unsupported internal features are represented as diagnostics when comparison
  can continue.
