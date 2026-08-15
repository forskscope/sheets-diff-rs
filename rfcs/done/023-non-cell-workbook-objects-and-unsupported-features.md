# RFC-023 — Non-Cell Workbook Objects and Unsupported Feature Reporting

**Status.** Implemented (2.0.0–2.2.3) — restored 2026-08-15; not individually re-verified against the implementation.
**Target:** v2.1+  
**Related:** RFC-005, RFC-014, RFC-016, RFC-021, RFC-022, RFC-032

## 1. Summary

Define how `sheets-diff` should treat workbook objects that are not ordinary
cells: tables, filters, merged cells, comments, hyperlinks, images, charts,
pivot tables, data validation, slicers, macros, and external links.

The core policy is: **do not silently imply full comparison coverage**. Unsupported
objects should be reported as diagnostics when detectable.

## 2. Motivation

A spreadsheet is not only a grid of values. In business workbooks, meaningful
changes often occur in filters, table definitions, comments, validation rules,
links, and charts. v2.0 does not need full object diffing, but the public result
should have a place for object-level changes and unsupported-feature warnings.

## 3. Goals

- Create an extensible object diff model.
- Allow v2.0 to emit unsupported-feature diagnostics.
- Avoid blocking core cell comparison on object support.
- Let applications show a coverage warning such as “cells compared; charts not
  compared.”

## 4. Non-goals

- Full OpenXML structural diff.
- Rendering or comparing chart images.
- VBA or macro code diff.
- Pivot cache semantic comparison.
- Image OCR or binary image similarity.

## 5. Public model

```rust
pub enum WorkbookObjectChange {
    Table(TableChange),
    AutoFilter(AutoFilterChange),
    MergedCellRange(MergedCellRangeChange),
    Comment(CommentChange),
    Hyperlink(HyperlinkChange),
    DataValidation(DataValidationChange),
    Chart(ChartChange),
    PivotTable(PivotTableChange),
    Unsupported(UnsupportedObjectChange),
}

pub struct UnsupportedObjectChange {
    pub object_kind: WorkbookObjectKind,
    pub location: Option<ObjectLocation>,
    pub message: String,
}
```

`WorkbookDiff` may contain:

```rust
pub object_changes: Vec<WorkbookObjectChange>
```

This field can be empty if object comparison is disabled.

## 6. Options

```rust
pub enum ObjectCompareMode {
    Ignore,
    WarnIfPresent,
    CompareAvailable,
}
```

Default for v2.0: `WarnIfPresent` if detection is cheap; otherwise `Ignore`.

`WarnIfPresent` is a valuable compromise: it does not attempt object diffs, but
it prevents a misleading “no differences” result when there are unsupported
objects that might matter.

## 7. Priority order

If implemented, object support should be staged:

1. merged cell ranges;
2. hyperlinks;
3. comments/notes;
4. tables and auto-filters;
5. data validation;
6. charts and pivot tables as unsupported diagnostics only;
7. external links as security/privacy-sensitive diagnostics.

## 8. Internal design

Create an `ObjectSnapshot` layer that is independent from cells and styles:

```rust
enum ObjectSnapshot {
    MergedRange { sheet: SheetId, range: CellRange },
    Hyperlink { sheet: SheetId, address: CellAddress, target: String },
    Comment { sheet: SheetId, address: CellAddress, text: String },
    Unsupported { kind: WorkbookObjectKind, location: Option<ObjectLocation> },
}
```

Use object-specific keys for matching. For example, comments match by sheet and
address; tables match by sheet and table name where available.

## 9. Diagnostics

If object comparison is disabled but objects are detected, emit a summary-level
diagnostic, not thousands of per-object warnings.

## 10. Acceptance criteria

- A workbook with a chart but no cell changes can produce a coverage warning.
- Merged-range changes can be represented without changing the cell diff model.
- Unsupported object kinds are non-fatal by default.
- Security-sensitive external links are not followed.
