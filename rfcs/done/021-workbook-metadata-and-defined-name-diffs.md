# RFC-021 — Workbook Metadata and Defined-Name Diffs

**Status.** Partially implemented (2.0.0–2.2.3) — verified 2026-08-16. Deferred: `WorkbookMetadataMode` (proposed to allow disabling metadata comparison) was never implemented — `compare_workbook_metadata`'s `_opts` parameter is unused and metadata comparison cannot be configured or disabled, though its own code comments incorrectly claim it can; defined-name and sheet-visibility diffing exist but have no test coverage.
**Target:** v2.1 candidate  
**Related:** RFC-003, RFC-005, RFC-014, RFC-023

## 1. Summary

Add optional support for comparing workbook-level metadata and defined names
(named ranges). These are important in real spreadsheets but should not block
v2.0 cell-diff stabilization.

## 2. Motivation

A workbook can change without any visible cell value changing:

- document properties may change;
- calculation settings may change;
- named ranges may be added, removed, or retargeted;
- workbook-level external links may change;
- sheet visibility may change.

For a GUI diff/merge application, these changes can matter. However, not all
underlying readers expose them uniformly. v2 should define an extensible model
that can be partially implemented with honest diagnostics.

## 3. Goals

- Represent metadata changes separately from cell changes.
- Represent defined-name additions, removals, and target changes.
- Report unsupported metadata categories as diagnostics.
- Keep the default v2.0 result usable even if metadata diffing is disabled.

## 4. Non-goals

- Full OpenXML package diff.
- Editing named ranges.
- Evaluating named formulas.
- Comparing VBA projects or macros.

## 5. Public model

```rust
pub struct WorkbookDiff {
    pub sheets: Vec<SheetDiff>,
    pub workbook_changes: Vec<WorkbookChange>,
    pub diagnostics: Vec<Diagnostic>,
    pub summary: DiffSummary,
}

pub enum WorkbookChange {
    PropertyChanged(PropertyChange),
    DefinedNameAdded(DefinedNameSnapshot),
    DefinedNameRemoved(DefinedNameSnapshot),
    DefinedNameChanged(DefinedNameChange),
    SheetVisibilityChanged(SheetVisibilityChange),
    CalculationModeChanged(CalculationModeChange),
}

pub struct DefinedNameSnapshot {
    pub name: String,
    pub scope: DefinedNameScope,
    pub target: String,
    pub hidden: Option<bool>,
}
```

`WorkbookChange` should be non-exhaustive if exposed publicly.

## 6. Options

```rust
pub enum WorkbookMetadataMode {
    Ignore,
    CompareAvailable,
    RequireSupported,
}
```

Default for v2.0: `Ignore` or `CompareAvailable` depending on implementation
confidence. Default for v2.1 may become `CompareAvailable` if stable.

`RequireSupported` returns an error if the workbook contains metadata categories
that the library cannot inspect. This is useful for audit workflows but too
strict for GUI defaults.

## 7. Internal design

Create an internal metadata extraction layer independent from cell extraction:

```rust
struct WorkbookMetadataSnapshot {
    properties: BTreeMap<PropertyKey, PropertyValue>,
    defined_names: BTreeMap<DefinedNameKey, DefinedNameSnapshot>,
    sheet_visibility: BTreeMap<SheetId, SheetVisibility>,
    calculation: Option<CalculationSettings>,
    unsupported: Vec<UnsupportedMetadataFeature>,
}
```

Use ordered maps for deterministic output.

## 8. Matching rules

Defined names match by `(scope, normalized_name)`. Target comparison defaults to
raw formula/reference string comparison. Semantic formula equivalence is out of
scope.

## 9. Diagnostics

If metadata diffing is requested but a category is unavailable, emit:

```rust
DiagnosticKind::UnsupportedWorkbookMetadata { category, severity }
```

Severity is `Warning` in `CompareAvailable` and `Error` in `RequireSupported`.

## 10. Acceptance criteria

- Added/removed/changed defined names are detected in fixtures.
- Sheet visibility changes can be represented even if not implemented in v2.0.
- Unsupported metadata is visible through diagnostics.
- Cell-diff output remains unchanged when metadata mode is `Ignore`.
