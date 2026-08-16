# RFC-006: Diff Options and Configuration Builder

**Status.** Implemented (2.0.0–2.2.3) — verified 2026-08-16 against the implementation.
**Target:** v2.0.0  
**Created:** 2026-06-11  
**Category:** Configuration  

## 1. Summary

Define `DiffOptions` as the stable control surface for comparison behavior, limits, diagnostics, and future modes.

## 2. Motivation

A single constructor with fixed behavior cannot support GUI, CLI, and batch use cases. v2 needs options, but options must not become an unstructured bag of flags. A builder provides a stable, discoverable surface while keeping defaults simple.

## 3. Goals

- Provide sensible defaults for simple users.
- Allow advanced callers to configure comparison behavior.
- Avoid breaking API changes when adding future options.
- Group related options by concern.
- Make default behavior deterministic and cheap.

## 4. Non-goals

- Do not expose every internal tuning knob.
- Do not make advanced alignment the default in v2.0.
- Do not require users to construct options for common comparison.

## 5. External design

Proposed API:

```rust
pub struct DiffOptions {
    pub formula_comparison: FormulaComparison,
    pub value_comparison: ValueComparison,
    pub sheet_matching: SheetMatchingMode,
    pub alignment: AlignmentMode,
    pub diagnostics: DiagnosticMode,
    pub bounds: Bounds,
    pub progress: ProgressOptions,
}

impl DiffOptions {
    pub fn builder() -> DiffOptionsBuilder;
}
```

Defaults:

```text
formula_comparison = CompareFormulaText
value_comparison   = TypedExact
sheet_matching     = ExactNameThenConservativeRename
alignment          = Positional
warnings           = Collect
bounds             = ReasonableButNonSurprisingDefaults
```

## 6. Internal design

The builder should validate combinations during `build()` where possible. Example: key-column alignment requires a key column spec. Invalid combinations return a configuration error before workbooks are opened.

Internally, options should be cloned cheaply or passed by shared reference through the pipeline.

## 7. Data lifecycle

1. Caller uses default compare API or builds options.
2. Options are validated.
3. Valid options are passed to source opening, normalization, sheet matching, cell comparison, progress, and output layers.
4. Result includes enough metadata to explain which important modes were used.

## 8. Error, diagnostic, and edge-case behavior

Invalid option combinations return `SheetsDiffError::InvalidOptions` or equivalent before I/O begins. Bounds violations during comparison return `LimitExceeded`.

## 9. Testing and acceptance criteria

Acceptance criteria:

- Default comparison requires no options.
- Builder examples compile.
- Invalid options are caught before workbook opening.
- Options are documented with defaults.
- Adding a new option in v2.x does not require changing common call sites.

## 10. Migration and compatibility

v1 callers had almost no configuration. Migration docs should show the default v2 call first, then advanced options.

## 11. Open questions

- Should `DiffOptions` fields be public or accessed through methods only?
- Should defaults be conservative or more feature-rich for rename detection?
