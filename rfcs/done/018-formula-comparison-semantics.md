# RFC-018 — Formula Comparison Semantics

**Status.** Implemented (2.0.0–2.2.3) — verified 2026-08-16 against the implementation.
**Target:** v2.0 decision, v2.0 implementation if small  
**Related:** RFC-003, RFC-006, RFC-007, RFC-010, RFC-014

## 1. Summary

Define how `sheets-diff` v2 compares formulas, how formula changes relate to
cached value changes, and what the library explicitly does not attempt to do.

The current v1 model can emit separate value and formula diffs for the same
cell. v2 must make this behavior intentional. Spreadsheet applications often
store both the formula expression and a cached calculated value. A robust diff
library must allow app developers to show either or both without implying that
`sheets-diff` recalculates formulas.

## 2. Motivation

A formula cell has at least two observable layers:

1. the formula expression, such as `=A1+B1`; and
2. the cached result value stored in the workbook, such as `42`.

Those two layers can change independently:

- the formula can change while the cached result remains the same;
- the cached result can change because input cells changed while the formula is
  unchanged;
- a workbook can contain stale cached values if it was not recalculated by the
  authoring application;
- formula text can differ while being semantically equivalent.

If v2 only exposes a single `CellDiffKind::Formula` or `CellDiffKind::Value`,
GUI integrations will either lose important information or need to reconstruct
semantics themselves.

## 3. Goals

- Represent formula expression changes explicitly.
- Represent cached value changes explicitly.
- Allow a cell to report both expression and cached-value changes.
- Do not claim to calculate or verify formula results.
- Preserve deterministic ordering and serialization.
- Provide enough metadata for GUI tools to render formula/value changes as one
  per-cell row if desired.

## 4. Non-goals

- Full Excel formula evaluation.
- Dependency graph recalculation.
- Semantic formula equivalence proof.
- Cross-workbook external reference resolution.
- Locale-specific formula parsing.
- Editing or repairing formulas.

## 5. Public model

Introduce a cell change model that distinguishes layers:

```rust
pub struct CellDiff {
    pub address: CellAddress,
    pub value: Option<ValueChange>,
    pub formula: Option<FormulaChange>,
    pub notes: Vec<CellDiagnostic>,
}

pub struct ValueChange {
    pub old: CellValue,
    pub new: CellValue,
    pub equality: ValueEquality,
}

pub struct FormulaChange {
    pub old: Option<FormulaText>,
    pub new: Option<FormulaText>,
    pub comparison: FormulaComparison,
}

pub struct FormulaText {
    pub raw: String,
    pub normalized: Option<String>,
}
```

`CellDiff` is one record per changed address by default. This avoids duplicate
address surprises while still exposing independent formula and value subchanges.
If a later compatibility mode wants legacy split records, it should be an output
adapter, not the primary model.

## 6. Options

```rust
pub enum FormulaCompareMode {
    RawText,
    NormalizedText,
    RawAndNormalized,
    Ignore,
}

pub struct DiffOptions {
    pub formula_compare: FormulaCompareMode,
    pub include_formula_cached_values: bool,
}
```

Default: `RawText` and `include_formula_cached_values = true`.

`NormalizedText` is allowed only if the implementation has a clearly documented
normalizer. Until then, the public enum can exist but the mode may return
`UnsupportedOption` unless the corresponding feature is enabled.

## 7. Internal design

### 7.1 Extraction

The workbook reader should extract formula text and cell data independently.
If the underlying calamine API does not expose formula text for some workbook
shape, emit a sheet/cell diagnostic and continue value comparison.

Internal normalized representation:

```rust
struct NormalizedCell {
    coord: Coord,
    value: CellValue,
    formula: Option<String>,
    source_kind: CellSourceKind,
}
```

### 7.2 Comparison

For each aligned coordinate:

1. compare formula presence and text according to `FormulaCompareMode`;
2. compare typed values according to `ValueComparePolicy`;
3. build one `CellDiff` if either layer changed;
4. attach diagnostics for unavailable formula text, stale-value suspicion, or
   unsupported formula features when detectable.

### 7.3 Stale cached values

`sheets-diff` should not infer stale cached values merely because formulas are
present. It may emit `DiagnosticKind::FormulaCachedValueUnverified` once per
workbook or sheet when formula cells are compared, unless the option suppresses
that warning.

## 8. Serialization

JSON should preserve both subchanges:

```json
{
  "address": "C3",
  "value": { "old": {"number": 41.0}, "new": {"number": 42.0} },
  "formula": { "old": "=A3+B3", "new": "=A3+B3+C3" }
}
```

A missing `value` means no value change. A missing `formula` means no formula
change. `null` inside `FormulaChange` means formula added or removed.

## 9. Acceptance criteria

- A formula-only change produces one `CellDiff` with `formula != None` and
  `value == None`.
- A cached-value-only change produces one `CellDiff` with `value != None` and
  `formula == None`.
- A formula and value change at the same address produces one `CellDiff` with
  both fields populated.
- The library never states that it recalculated formulas.
- CLI output can render formula and value subchanges in a readable form.
- JSON schema fixtures cover formula-only, value-only, both, formula added, and
  formula removed cases.

## 10. Risks

Formula support depends on what the `.xlsx` reader exposes. If the reader cannot
extract formula text in all cases, diagnostics must be honest. A partially
supported feature is acceptable only if the result makes unsupported portions
visible to consumers.
