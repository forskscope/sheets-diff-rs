# RFC-007: Typed Cell Values and Normalization

**Status.** Implemented (2.0.0–2.2.3) — restored 2026-08-15; not individually re-verified against the implementation.
**Target:** v2.0.0  
**Created:** 2026-06-11  
**Category:** Cell model  

## 1. Summary

Preserve spreadsheet cell value types instead of collapsing all values to display strings.

## 2. Motivation

The central value of a spreadsheet-aware diff is that it knows spreadsheet values are not plain text. v1 stringifies values, making numeric/text/date/error distinctions impossible for applications. v2 must preserve typed values while still offering display strings for convenience.

## 3. Goals

- Represent empty, text, number, boolean, date/time, duration, error, and unsupported values.
- Preserve enough information for app-side formatting.
- Avoid exposing calamine as the only public representation.
- Provide stable equality semantics for diffing.
- Provide convenience display rendering without making it canonical.

## 4. Non-goals

- Do not evaluate formulas.
- Do not implement locale-specific Excel formatting in v2.0.
- Do not guarantee perfect round-trip to original workbook formatting.

## 5. External design

Proposed public value model:

```rust
pub enum CellValue {
    Empty,
    Text(String),
    Number(NumberValue),
    Bool(bool),
    DateTime(DateTimeValue),
    Duration(DurationValue),
    Error(CellErrorValue),
    Unsupported { display: String, reason: String },
}

pub struct DisplayValue {
    pub text: String,
    pub source: DisplaySource,
}
```

`NumberValue` should preserve the original numeric category where practical:

```rust
pub enum NumberValue {
    Int(i64),
    Float(f64),
}
```

If calamine exposes only a specific representation for some cells, normalize conservatively and preserve display text.

## 6. Internal design

Internal normalization:

```rust
fn normalize_cell(data: &calamine::Data) -> NormalizedValue {
    match data {
        Data::Empty => NormalizedValue::Empty,
        Data::String(s) => NormalizedValue::Text(s.clone()),
        Data::Float(f) => NormalizedValue::Number(NumberValue::Float(*f)),
        Data::Int(i) => NormalizedValue::Number(NumberValue::Int(*i)),
        Data::Bool(b) => NormalizedValue::Bool(*b),
        Data::Error(e) => NormalizedValue::Error(...),
        other => NormalizedValue::Unsupported { display: other.to_string(), ... },
    }
}
```

Comparison should use normalized values, not display strings. Display strings are derived after comparison.

## 7. Data lifecycle

1. Sheet reader yields raw calamine cells.
2. Raw cells are converted into `NormalizedCell`.
3. `NormalizedCell` stores value and formula data separately.
4. Comparison uses typed equality.
5. Public `CellValue` and display fields are emitted.

## 8. Error, diagnostic, and edge-case behavior

Unsupported or lossy conversions should generate diagnostics only when the consumer may care. For example, an unrecognized calamine variant should become `Unsupported` rather than panic.

Floating point comparison defaults to exact representation from parser. Future options may add tolerance, but v2.0 should not silently use tolerance.

## 9. Testing and acceptance criteria

Acceptance criteria:

- Text `"100"` and number `100` compare as different values.
- Boolean `TRUE` and text `"TRUE"` compare as different values.
- Error cells and text that looks like an error compare as different values.
- Date/time fixtures preserve type information where calamine provides it.
- Display strings are available but not used as canonical equality.

## 10. Migration and compatibility

This is a breaking change from `Option<String>`. Migration docs should show how to call `value.display_text()` or equivalent for consumers that still want strings.

## 11. Open questions

- Should date/time use chrono types, calamine-compatible serials, or a custom enum?
- Should number formatting metadata be included in v2.0 or deferred?
