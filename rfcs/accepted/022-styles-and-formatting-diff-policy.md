# RFC-022 — Styles and Formatting Diff Policy

**Status.** Accepted — design settled; implementation incomplete as of 2.2.3. See ../README.md.
**Target:** v2.1 candidate  
**Related:** RFC-020, RFC-023, RFC-029

## 1. Summary

Define an optional formatting diff layer for cell styles, number formats,
fill/border/font attributes, alignment, and related visual metadata. This RFC
keeps formatting out of the default v2.0 value-diff path while reserving a clean
extension model.

## 2. Motivation

Some spreadsheet diffs care only about data. Others care about review/audit
changes where formatting is meaningful: red cells, locked cells, hidden rows,
number format changes, or conditional formatting. If formatting is mixed into
ordinary `CellDiff` by default, the output becomes noisy. If formatting is not
modeled at all, later additions may break the API.

## 3. Goals

- Provide an optional style diff model.
- Keep value/formula comparison independent from formatting by default.
- Make formatting support partial and diagnostic-aware.
- Avoid promising Excel-perfect visual rendering.

## 4. Non-goals

- Pixel rendering.
- Conditional formatting formula evaluation.
- Theme resolution identical to Excel.
- Merge/write support.

## 5. Public model

```rust
pub struct CellDiff {
    pub address: CellAddress,
    pub value: Option<ValueChange>,
    pub formula: Option<FormulaChange>,
    pub format: Option<FormatChange>,
    pub notes: Vec<CellDiagnostic>,
}

pub struct FormatChange {
    pub old: Option<CellFormatSnapshot>,
    pub new: Option<CellFormatSnapshot>,
    pub changed_fields: Vec<FormatField>,
}

pub struct CellFormatSnapshot {
    pub number_format: Option<CellNumberFormat>,
    pub font: Option<FontFormat>,
    pub fill: Option<FillFormat>,
    pub border: Option<BorderFormat>,
    pub alignment: Option<AlignmentFormat>,
    pub protection: Option<ProtectionFormat>,
}
```

All fields are optional because reader support may be incomplete.

## 6. Options

```rust
pub enum FormatCompareMode {
    Ignore,
    NumberFormatOnly,
    BasicStyle,
    AllAvailable,
}
```

Default: `Ignore`.

`NumberFormatOnly` may be implemented earlier because it has direct interaction
with display and typed value interpretation. `AllAvailable` is best-effort and
must attach diagnostics for unsupported style categories.

## 7. Internal design

Style snapshots should be interned to reduce memory use:

```rust
struct StyleTable {
    formats: Vec<CellFormatSnapshot>,
    ids: HashMap<CellFormatSnapshot, StyleId>,
}

struct NormalizedCell {
    coord: Coord,
    value: CellValue,
    formula: Option<String>,
    style: Option<StyleId>,
}
```

Comparing style IDs is cheap. Detailed field differences are computed only when
style IDs differ and formatting comparison is enabled.

## 8. Conditional formatting

Conditional formatting should not be evaluated. A future object-diff layer may
report that conditional formatting rules changed, but cell-level format changes
that depend on evaluated conditions are out of scope.

## 9. UX guidance

Formatting diffs should be displayed separately or behind a toggle in GUI tools.
A default table that mixes value and style changes can become too noisy.

## 10. Acceptance criteria

- Default v2 output is unchanged by style-only changes.
- With `NumberFormatOnly`, a number-format change is reported without pretending
  the cell value changed.
- Style support is clearly documented as best-effort.
- Unsupported style features produce diagnostics when style comparison is
  requested.
