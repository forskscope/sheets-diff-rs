# RFC-006: Diff Options and Configuration Builder

**Status.** Implemented (2.0.0–2.2.3) — verified 2026-08-16 against the implementation.

**Note added M10 unit 08 (unreleased): the options tree is extensible, and a new option is additive from 3.0.0 on.**
All eight options structs — `DiffOptions`, `ComparisonOptions`, `ValueCompareOptions`, `MatchingOptions`, `Limits`,
`ExecutionOptions`, `DiagnosticOptions`, `OutputOptions` — are `#[non_exhaustive]`, exactly as every result struct always
was. **An author planning a new option can add a field without a breaking change**: the builder gets a setter (the
coverage audit in `src/builder_coverage.rs` fails to compile until it is listed), and no caller's code stops compiling,
because no caller outside the crate can name every field. Through 2.x this was not so — a caller could write a struct
literal naming every field, so every option was a break — which is why RFC-011's column alignment, RFC-022's formatting and
a formula normaliser each risked waiting for a major. Callers build options with the builder, or with `Default::default()`
and field assignment; **`..Default::default()` is not available from outside the crate** (functional update is a struct
expression, rejected exactly as a full literal is).

**Note added M10 unit 03 (unreleased):** the `DiffOptions` struct in §5 (`formula_comparison`,
`value_comparison`, `bounds`, …) is the pre-implementation *sketch*; the implemented option tree is
RFC-033 §11's (`comparison`, `matching`, `limits`, `execution`, `diagnostics`, `output`), and that section is
authoritative for it, including the builder's final coverage and naming rule. The builder this RFC calls for
covers every option as of 3.0.0 (`DiffOptionsBuilder` has a method for each of its nineteen leaves, asserted by
`tests/builder_coverage.rs`).
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
