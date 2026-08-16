# RFC-019 — Numeric, Date, and Tolerance Comparison Policies

**Status.** Partially implemented (2.0.0–2.2.3) — verified 2026-08-16. Deferred: `CellValue::Duration` values are unreachable through any `.xlsx` input this crate accepts, so duration-tolerance comparison is unexercised in practice.
**Target:** v2.0 decision  
**Related:** RFC-006, RFC-007, RFC-010, RFC-014

## 1. Summary

Define explicit comparison policies for numbers, floating-point values, dates,
datetimes, durations, and typed-vs-text differences. Spreadsheet diffs are used
by humans, so exact machine equality is not always the desired behavior; however,
implicit tolerance is dangerous. v2 should default to exact typed comparison and
allow opt-in tolerance policies.

## 2. Motivation

The v1 implementation stringifies values. That hides important distinctions:
text `"100"` and number `100` can look identical after display conversion, and
date serial values can be difficult to distinguish from ordinary numbers.

v2 typed values solve representation, but comparison still needs policy:

- Should `1` and `1.0` be equal?
- Should `0.30000000000000004` and `0.3` be equal?
- Should date serial `45292` equal ISO date `2024-01-01` if the reader exposes
  them differently?
- Should text `"2024-01-01"` equal a date cell?

The default must be safe, deterministic, and explainable.

## 3. Goals

- Define default exact typed comparison.
- Allow optional numeric tolerance.
- Allow optional date/datetime normalization where type information is reliable.
- Never silently coerce text into numbers or dates by default.
- Preserve the reason a value was considered equal or different.

## 4. Non-goals

- Locale-aware parsing of text into dates or numbers.
- Spreadsheet recalculation.
- Number-format-aware semantic equality in v2.0.
- Arbitrary precision decimal engine as a required dependency.

## 5. Public API

```rust
pub struct ValueCompareOptions {
    pub number_policy: NumberComparePolicy,
    pub date_policy: DateComparePolicy,
    pub type_mismatch_policy: TypeMismatchPolicy,
}

pub enum NumberComparePolicy {
    Exact,
    AbsoluteTolerance(f64),
    RelativeTolerance(f64),
    AbsoluteOrRelative { abs: f64, rel: f64 },
}

pub enum DateComparePolicy {
    ExactRepresentation,
    NormalizeEquivalentDateTimes,
}

pub enum TypeMismatchPolicy {
    Different,
    CompareDisplayString,
}
```

Defaults:

```rust
NumberComparePolicy::Exact
DateComparePolicy::ExactRepresentation
TypeMismatchPolicy::Different
```

`CompareDisplayString` is intended for human-friendly reports but should be
clearly marked as a display-mode comparison, not a structured-data comparison.

## 6. Result metadata

`ValueChange` should include why values differ:

```rust
pub enum ValueDifferenceKind {
    TypeChanged,
    ContentChanged,
    NumericOutsideTolerance,
    DateTimeChanged,
    ErrorKindChanged,
    DisplayStringChanged,
}
```

When tolerance causes two values to be treated as equal, no `ValueChange` is
emitted by default. If an application wants suppressed near-equality metadata,
that belongs in a later `explain_equalities` mode, not the v2.0 core.

## 7. Internal design

### 7.1 Numeric comparison

- `Int` vs `Int`: exact integer equality.
- `Float` vs `Float`: policy-driven.
- `Int` vs `Float`: default type mismatch unless an option explicitly allows
  numeric cross-type comparison. This avoids losing type-safety.

Potential future extension:

```rust
pub enum NumericTypePolicy {
    PreserveType,
    CompareMathematicalValue,
}
```

This should not be the default.

### 7.2 Date/datetime comparison

The internal model should preserve the source variant:

```rust
pub enum CellValue {
    Empty,
    Text(String),
    Number(f64),
    Integer(i64),
    Bool(bool),
    DateTime(DateTimeValue),
    Duration(DurationValue),
    Error(CellErrorValue),
}
```

`DateTimeValue` should retain enough source metadata to avoid pretending that an
ambiguous serial number is certainly a date.

### 7.3 Type mismatch

Text should not be parsed to number/date by default. GUI applications may choose
an alternate render mode, but the library should not hide a type change.

## 8. Acceptance criteria

- Numeric tolerance is opt-in and covered by tests.
- Default mode reports number `100` vs text `"100"` as a type change.
- Default mode reports integer `1` vs float `1.0` according to the chosen typed
  representation policy and documents it.
- Date/datetime fixtures include ISO dates, spreadsheet serials where exposed,
  and duration values.
- JSON output includes enough type data for consumers to reproduce the same
  comparison result.

## 9. Risks

Tolerances can hide real changes. For this reason, tolerance must be visible in
`DiffOptions`, documented in report metadata, and never enabled by default.
