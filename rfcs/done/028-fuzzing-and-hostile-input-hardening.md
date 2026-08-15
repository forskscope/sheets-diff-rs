# RFC-028 — Fuzzing and Hostile-Input Hardening

**Status.** Implemented (2.0.0–2.2.3) — restored 2026-08-15; not individually re-verified against the implementation.
**Target:** v2.0+  
**Related:** RFC-004, RFC-005, RFC-016, RFC-026

## 1. Summary

Define a fuzzing and hostile-input strategy for `.xlsx` input handling. A diff
library embedded in GUI applications must treat malformed files as ordinary
untrusted input, not as exceptional process-crashing events.

## 2. Motivation

`.xlsx` files are ZIP packages containing XML and related parts. They may be
corrupt, truncated, maliciously large, password-protected, or crafted to trigger
parser edge cases. Even if low-level parsing is delegated to `calamine`,
`sheets-diff` owns its error handling, resource limits, and no-panic contract.

## 3. Goals

- Verify malformed inputs return errors or diagnostics, not panics.
- Exercise path, bytes, and reader APIs.
- Test resource-limit behavior.
- Add fuzz targets for address conversion, range merging, typed value
  normalization, and workbook opening where feasible.

## 4. Non-goals

- Replacing calamine's parser.
- Guaranteeing safety against all decompression bombs in v2.0.
- Opening encrypted workbooks.

## 5. Fuzz targets

Suggested targets:

```text
fuzz_addr_roundtrip
fuzz_range_merge
fuzz_cell_value_normalization
fuzz_diff_options_builder
fuzz_open_xlsx_bytes
fuzz_sheet_matching_manifest
```

`fuzz_open_xlsx_bytes` should be behind an optional fuzzing setup because parser
fuzzing can be expensive.

## 6. Corpus seeds

Seed corpus should include:

- empty file;
- random bytes;
- valid ZIP but not XLSX;
- truncated XLSX;
- password-protected workbook if fixture licensing allows;
- workbook with empty sheets;
- workbook with wide columns;
- workbook with many sheets;
- workbook with formulas;
- workbook with unsupported objects.

## 7. Panic policy

Public APIs must not panic on malformed input. Panics in internal debug asserts
are acceptable only when unreachable by public ordinary input and should not be
used for parser errors.

Use `Result` and diagnostics consistently.

## 8. Resource hardening

Fuzz and tests should cover:

- maximum sheet count;
- maximum cell count;
- maximum returned diff count;
- cancellation during long comparison;
- large shared string tables if the reader exposes them.

## 9. CI integration

- Normal CI runs regression fixtures.
- Fuzz targets compile in CI.
- Nightly/manual job runs fuzzing for a time budget.
- Crashes create minimized corpus entries.

## 10. Acceptance criteria

- Malformed bytes through `try_from_bytes` never panic in regression tests.
- Address/range fuzz targets run without panics.
- At least one fuzz target is documented for maintainers.
- Security policy states that files are untrusted input and external links are
  never followed.
