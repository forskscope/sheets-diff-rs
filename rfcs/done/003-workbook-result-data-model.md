# RFC-003: Workbook Result Data Model

**Status.** Implemented (2.0.0–2.2.3) — verified 2026-08-16 against the implementation.
**Target:** v2.0.0  
**Created:** 2026-06-11  
**Category:** Data model  

## 1. Summary

Define the structured `WorkbookDiff`, `SheetDiff`, `CellDiff`, summaries, and identity model returned by v2 APIs.

## 2. Motivation

The current string-oriented model is too narrow for app developers. GUI and CLI consumers need stable, typed data with sheet-level and cell-level structure. This RFC defines the canonical output model from which text, JSON, and UI views can be derived.

## 3. Goals

- Represent workbook, sheet, and cell changes explicitly.
- Make sheet and cell ordering deterministic.
- Include summaries without requiring consumers to recompute basic counts.
- Allow added/removed/renamed sheets and changed cells to coexist in one result.
- Avoid leaking absolute source paths unless provided as display labels.

## 4. Non-goals

- Do not model every Excel style or workbook property in v2.0.
- Do not represent formulas as evaluated expression trees.
- Do not include merge decisions or patch application data.

## 5. External design

Core types:

```rust
pub struct WorkbookDiff {
    pub old: WorkbookSide,
    pub new: WorkbookSide,
    pub sheets: Vec<SheetDiff>,
    pub diagnostics: Vec<Diagnostic>,
    pub summary: DiffSummary,
}

pub struct SheetDiff {
    pub old_sheet: Option<SheetRef>,
    pub new_sheet: Option<SheetRef>,
    pub change: SheetChange,
    pub cell_diffs: Vec<CellDiff>,
    pub compared_range: ComparedRange,
    pub diagnostics: Vec<Diagnostic>,
    pub summary: SheetSummary,
}

pub struct CellDiff {
    pub address: CellAddress,
    pub value_change: Option<ValueChange>,
    pub formula_change: Option<FormulaChange>,
    pub diagnostics: Vec<Diagnostic>,
}
```

`CellDiff` is intentionally merged per address. A cell whose value and formula both changed is represented by one `CellDiff` with two optional subchanges.

## 6. Internal design

Internal comparison should build a normalized intermediate representation before constructing public model objects:

```rust
struct NormalizedWorkbook {
    side: Side,
    sheets: Vec<NormalizedSheet>,
}

struct NormalizedSheet {
    name: String,
    index: usize,
    used_range: Option<CellRange>,
    cells: BTreeMap<CellCoord, NormalizedCell>,
}
```

The public model should be constructed after sorting and diagnostics aggregation so that consumers receive deterministic data.

## 7. Data lifecycle

1. Input source is opened.
2. Workbook metadata is read.
3. Sheets are normalized into internal sheet models.
4. Sheet pairs are matched.
5. Cell changes are computed.
6. Diagnostics and summaries are aggregated.
7. A `WorkbookDiff` is returned.

## 8. Error, diagnostic, and edge-case behavior

If a sheet cannot be read but the workbook can still be inspected, behavior depends on options:

- strict mode: return `SheetsDiffError::ReadSheet`;
- lenient mode: include sheet-level diagnostic and continue where possible.

The model must be able to contain diagnostics at workbook, sheet, and cell granularity.

## 9. Testing and acceptance criteria

Acceptance criteria:

- Added, removed, unchanged, changed, and renamed sheet fixtures are represented.
- A cell with both value and formula changes produces one `CellDiff`.
- Summary counts match the actual model contents.
- Sorting is stable across repeated runs.
- Model examples compile in documentation tests.

## 10. Migration and compatibility

This is a breaking change. v1 consumers mapping `Diff { sheets, cells }` must migrate to `WorkbookDiff { sheets }` and nested `SheetDiff::cell_diffs`.

Provide a migration example that flattens v2 output into a v1-like list for simple applications.

## 11. Open questions

- Should all public structs be non-exhaustive to preserve v2.x evolution?
- Should `CellDiff` include both zero-based and one-based coordinates, or only one-based Excel-style coordinates?
