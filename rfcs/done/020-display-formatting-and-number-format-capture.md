# RFC-020 — Display Formatting and Number-Format Capture

**Status.** Partially implemented (2.0.0–2.2.3) — verified 2026-08-16. Deferred: `CellNumberFormat` is always `None`; the number-format capture described in the design was never implemented.
**Target:** v2.0 decision, v2.1 implementation candidate  
**Related:** RFC-003, RFC-007, RFC-014, RFC-022

## 1. Summary

Separate typed value comparison from display formatting. v2 should preserve a
safe display string for convenience, but it must not make display formatting the
only representation. Number formats and display strings should be captured as
optional metadata when available.

## 2. Motivation

GUI diff tools need two things at once:

1. reliable structured values for comparison and filtering; and
2. human-friendly text for table cells, summaries, and tooltips.

The v1 implementation stores only `Data::to_string()`. That makes display easy
but loses type information. v2 should invert the relationship: typed value is
primary, display is derived or captured as metadata.

## 3. Goals

- Keep `CellValue` typed.
- Provide a stable display helper for app developers.
- Capture workbook number format identifiers or format strings when available.
- Allow applications to show raw typed value, formatted display, or both.
- Avoid locale-sensitive rendering in the core by default.

## 4. Non-goals

- Pixel-perfect Excel rendering.
- Full Excel number-format engine in v2.0.
- Locale auto-detection.
- Style/font/color comparison; that is covered by RFC-022.

## 5. Public model

```rust
pub struct CellSnapshot {
    pub value: CellValue,
    pub formula: Option<FormulaText>,
    pub display: Option<CellDisplay>,
}

pub struct CellDisplay {
    pub text: String,
    pub format: Option<CellNumberFormat>,
    pub source: DisplaySource,
}

pub struct CellNumberFormat {
    pub id: Option<u32>,
    pub code: Option<String>,
}

pub enum DisplaySource {
    ReaderProvided,
    SheetsDiffDefault,
    ApplicationProvided,
}
```

In v2.0, `CellDisplay` may be omitted if the reader cannot provide reliable
formatting. `CellValue::display_default()` should return a deterministic,
locale-neutral string.

## 6. API design

```rust
impl CellValue {
    pub fn display_default(&self) -> String;
}

impl CellSnapshot {
    pub fn preferred_display(&self) -> String;
}
```

`preferred_display()` uses `display.text` when present and falls back to
`value.display_default()`.

## 7. Internal design

- During workbook reading, capture the typed value and any format metadata the
  underlying reader exposes.
- Do not attempt to synthesize complex Excel formatting unless a dedicated
  feature is enabled.
- Store display metadata in snapshots, not only in diffs, so added/removed
  values have the same shape as changed values.

## 8. Diff behavior

By default, value comparison ignores display metadata. A number format change
without a value change is not a `ValueChange`; it belongs to an optional format
or style diff path. This avoids surprising users who asked for data comparison.

Optional future mode:

```rust
pub enum DisplayCompareMode {
    Ignore,
    CompareDisplayText,
    CompareNumberFormat,
}
```

This should be introduced only after RFC-022 is accepted.

## 9. Serialization

When the `serde` feature is enabled, display metadata must be serialized as
optional fields so older v2 consumers can ignore it without losing the primary
value model.

## 10. Acceptance criteria

- Typed value fixtures still pass when display metadata is absent.
- Number `100` and text `"100"` remain distinguishable even if both display as
  `100`.
- Display helper output is deterministic across operating systems.
- The docs clearly state that v2.0 does not promise Excel-identical formatting.
