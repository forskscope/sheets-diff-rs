# RFC-029 — GUI Integration View Adapters

**Status.** Implemented (2.0.0–2.2.3) — verified 2026-08-16 against the implementation.
**Target:** v2.1 candidate  
**Related:** RFC-003, RFC-009, RFC-010, RFC-011, RFC-012, RFC-014

## 1. Summary

Add optional adapter types that help GUI applications present `WorkbookDiff`
without exposing GUI-framework-specific dependencies. The core result remains
framework-neutral; adapters provide navigation, grouping, filtering, and summary
views.

## 2. Motivation

Downstream GUI tools need common presentation structures:

- workbook summary;
- sheet tree;
- list of changed cells;
- navigation to next/previous change;
- filtering by change kind;
- grouped view by sheet, row, or address;
- stable anchors for virtualized tables.

Every app can build these independently, but a lightweight adapter layer reduces
integration friction and encourages consistent semantics.

## 3. Goals

- Provide GUI-friendly, framework-neutral view models.
- Keep the core model unchanged.
- Avoid dependencies on Dioxus, Iced, egui, Tauri, or web frameworks.
- Support virtualized table rendering through stable row IDs and anchors.

## 4. Non-goals

- Building a GUI inside `sheets-diff`.
- Providing merge UI.
- Owning app-specific state management.

## 5. Public API

```rust
pub struct DiffView<'a> {
    pub workbook: &'a WorkbookDiff,
}

pub struct SheetChangeSummaryView<'a> { /* borrowed fields */ }

pub struct CellChangeRow<'a> {
    pub id: ChangeAnchor,
    pub sheet_name: &'a str,
    pub address: CellAddress,
    pub row: u32,
    pub col: u32,
    pub kinds: CellChangeKinds,
    pub old_display: Option<String>,
    pub new_display: Option<String>,
    pub severity: ChangeSeverity,
}

pub struct ChangeAnchor {
    pub sheet_index: usize,
    pub row: u32,
    pub col: u32,
    pub subkind: Option<CellSubChangeKind>,
}
```

Adapters should borrow from `WorkbookDiff` where possible and allocate display
strings only when requested.

## 6. Filtering

```rust
pub struct ViewFilter {
    pub include_values: bool,
    pub include_formulas: bool,
    pub include_formatting: bool,
    pub include_diagnostics: bool,
    pub sheets: Option<Vec<SheetId>>,
}
```

Filtering should not mutate the underlying diff.

## 7. Navigation

Provide deterministic navigation over visible changes:

```rust
impl<'a> DiffView<'a> {
    pub fn rows(&self, filter: &ViewFilter) -> impl Iterator<Item = CellChangeRow<'a>>;
    pub fn next_after(&self, anchor: &ChangeAnchor, filter: &ViewFilter) -> Option<ChangeAnchor>;
    pub fn previous_before(&self, anchor: &ChangeAnchor, filter: &ViewFilter) -> Option<ChangeAnchor>;
}
```

## 8. Internal design

Adapters can be implemented as borrowed iterators over sorted `WorkbookDiff`.
If performance requires indexing, build an optional `DiffIndex`:

```rust
pub struct DiffIndex {
    by_sheet: Vec<SheetIndex>,
    anchors: Vec<ChangeAnchor>,
}
```

Index construction is explicit so callers control memory cost.

## 9. Acceptance criteria

- A GUI app can render a sheet tree and changed-cell table without inspecting
  internal fields beyond public adapters.
- Adapters have no GUI framework dependency.
- Navigation order matches JSON/CLI deterministic order.
- Virtualized table rows have stable anchors.
