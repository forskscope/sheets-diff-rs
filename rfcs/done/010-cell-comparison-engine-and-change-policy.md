# RFC-010: Cell Comparison Engine and Change Policy

**Status.** Implemented (2.0.0–2.2.3) — restored 2026-08-15; not individually re-verified against the implementation.
**Target:** v2.0.0  
**Created:** 2026-06-11  
**Category:** Comparison core  

## 1. Summary

Define how values, formulas, empty cells, added/removed cells, and duplicate-address changes are compared and represented.

## 2. Motivation

The v1 engine runs separate value and formula passes and can produce duplicate address entries. v2 should define a single per-cell result policy while preserving both value and formula changes.

## 3. Goals

- Compare normalized typed values, not display strings.
- Compare formula text separately from cached value.
- Represent one public `CellDiff` per address.
- Make empty/missing cell semantics explicit.
- Keep positional comparison as the default engine.

## 4. Non-goals

- Do not evaluate formulas.
- Do not use fuzzy value matching by default.
- Do not make row/column alignment mandatory in this RFC.

## 5. External design

Public change model:

```rust
pub struct CellDiff {
    pub address: CellAddress,
    pub value_change: Option<ValueChange>,
    pub formula_change: Option<FormulaChange>,
    pub kind: CellChangeKind,
    pub diagnostics: Vec<Diagnostic>,
}

pub enum CellChangeKind {
    Added,
    Removed,
    Modified,
}

pub struct ValueChange {
    pub old: CellValue,
    pub new: CellValue,
}

pub struct FormulaChange {
    pub old: Option<String>,
    pub new: Option<String>,
}
```

## 6. Internal design

Internal algorithm for positional mode:

```text
for each matched sheet pair:
  compute compared range
  for each coord in range:
    old_cell = old.cells.get(coord).unwrap_or(Empty)
    new_cell = new.cells.get(coord).unwrap_or(Empty)
    value_change = compare values
    formula_change = compare formulas
    if either changed:
      push one CellDiff
```

If the sheet is one-sided, generate added/removed cell diffs from the non-empty side.

Formula comparison should support modes:

```rust
pub enum FormulaComparison {
    Ignore,
    CompareFormulaText,
}
```

## 7. Data lifecycle

1. Sheet pair is selected.
2. Compared range is computed.
3. Normalized cells are looked up by coordinate.
4. Value and formula comparisons produce optional subchanges.
5. Subchanges are merged into one `CellDiff` per coordinate.
6. Diff list is sorted numerically.

## 8. Error, diagnostic, and edge-case behavior

Cells outside the used range are not compared. Empty-vs-empty produces no diff.

If formula text is unavailable but value exists, the result should not fabricate a formula diff. If cached formula value changes without formula text changing, that is a value change.

## 9. Testing and acceptance criteria

Acceptance criteria:

- Value-only changes produce one `CellDiff`.
- Formula-only changes produce one `CellDiff`.
- Value+formula changes produce one `CellDiff` with both subchanges.
- Empty-to-value and value-to-empty are represented as added/removed or modified according to policy.
- Formula comparison can be disabled.

## 10. Migration and compatibility

v1 consumers that expected multiple entries for the same address must adapt. The migration guide should include flattening helpers if a UI wants separate rows for value and formula changes.

## 11. Open questions

- Should empty-to-value be `Added` at cell level or `Modified` from `Empty` to value?
- Should formula text normalization ignore leading `=` or whitespace?
