use super::*;
use crate::model::{CellDuration, CellValue, DateTimeKind};
use crate::options::{DateComparePolicy, ValueCompareOptions};

fn opts() -> ValueCompareOptions {
    ValueCompareOptions::default()
}

fn iso_only_dt(iso: &str) -> CellDateTime {
    CellDateTime {
        serial: 0.0,
        is_1904: false,
        kind: DateTimeKind::DateTime,
        iso: Some(iso.to_string()),
        has_serial: false,
    }
}

fn serial_dt(serial: f64, is_1904: bool) -> CellDateTime {
    CellDateTime {
        serial,
        is_1904,
        kind: DateTimeKind::DateTime,
        iso: None,
        has_serial: true,
    }
}

// D-01: ISO date/time and duration values must not always compare equal ---

#[test]
fn iso_only_datetimes_with_same_iso_are_equal() {
    let a = iso_only_dt("2024-01-01T00:00:00");
    let b = iso_only_dt("2024-01-01T00:00:00");
    assert!(compare_values(&CellValue::DateTime(a), &CellValue::DateTime(b), &opts()).is_none());
}

#[test]
fn iso_only_datetimes_with_different_iso_are_reported_changed() {
    // The exact pair from the RFC-035 Handoff 05 audit: before the fix,
    // both normalise to serial 0.0 / is_1904 false / kind DateTime, so
    // this compared equal no matter how different the two dates are.
    let a = iso_only_dt("2024-01-01T00:00:00");
    let b = iso_only_dt("2099-12-31T23:59:59");
    let r = compare_values(&CellValue::DateTime(a), &CellValue::DateTime(b), &opts()).unwrap();
    assert_eq!(r.reason, ValueDifferenceKind::DateTimeChanged);
}

#[test]
fn iso_only_durations_with_different_iso_are_reported_changed() {
    // The second pair from the same audit finding.
    let a = CellValue::Duration(CellDuration {
        serial: 0.0,
        iso: Some("PT1H".to_string()),
    });
    let b = CellValue::Duration(CellDuration {
        serial: 0.0,
        iso: Some("PT99H".to_string()),
    });
    let r = compare_values(&a, &b, &opts()).unwrap();
    assert_eq!(r.reason, ValueDifferenceKind::ContentChanged);
}

#[test]
fn iso_only_durations_with_same_iso_are_equal() {
    let a = CellValue::Duration(CellDuration {
        serial: 0.0,
        iso: Some("PT1H30M".to_string()),
    });
    let b = CellValue::Duration(CellDuration {
        serial: 0.0,
        iso: Some("PT1H30M".to_string()),
    });
    assert!(compare_values(&a, &b, &opts()).is_none());
}

#[test]
fn mixed_serial_and_iso_datetime_never_silently_equal() {
    // A genuine serial-based value whose serial happens to be 0.0 (a
    // legitimate date, 1899-12-30 in the 1900 system) against an
    // ISO-only value whose serial is *also* 0.0 but as a placeholder.
    // Before `has_serial`, these were indistinguishable and compared
    // equal under the old (serial, is_1904, kind) check.
    let serial_based = serial_dt(0.0, false);
    let iso_only = iso_only_dt("2024-01-01T00:00:00");
    let mut o = opts();

    o.date = DateComparePolicy::ExactRepresentation;
    let r = compare_values(
        &CellValue::DateTime(serial_based.clone()),
        &CellValue::DateTime(iso_only.clone()),
        &o,
    );
    assert!(
        r.is_some(),
        "mixed representation must not be silently equal"
    );

    // Must not become equal under the normalisation policy either.
    o.date = DateComparePolicy::NormalizeEquivalentDateTimes;
    let r = compare_values(
        &CellValue::DateTime(serial_based),
        &CellValue::DateTime(iso_only),
        &o,
    );
    assert!(
        r.is_some(),
        "mixed representation must not be silently equal under NormalizeEquivalentDateTimes either"
    );
}

#[test]
fn normalize_equivalent_datetimes_reconciles_1900_and_1904_systems() {
    // Same real-world date, represented once under the 1900 system and
    // once under the 1904 system: serials differ by exactly the 1462-day
    // offset. `ExactRepresentation` must see them as different;
    // `NormalizeEquivalentDateTimes` must see them as the same instant.
    let system_1900 = serial_dt(45000.0, false);
    let system_1904 = serial_dt(45000.0 - 1462.0, true);

    let mut o = opts();
    o.date = DateComparePolicy::ExactRepresentation;
    let r = compare_values(
        &CellValue::DateTime(system_1900.clone()),
        &CellValue::DateTime(system_1904.clone()),
        &o,
    );
    assert!(
        r.is_some(),
        "ExactRepresentation must not conflate the two epochs"
    );

    o.date = DateComparePolicy::NormalizeEquivalentDateTimes;
    let r = compare_values(
        &CellValue::DateTime(system_1900),
        &CellValue::DateTime(system_1904),
        &o,
    );
    assert!(
        r.is_none(),
        "NormalizeEquivalentDateTimes must recognise the same instant across epochs"
    );
}

#[test]
fn equal_texts_produce_no_change() {
    let r = compare_values(
        &CellValue::Text("x".into()),
        &CellValue::Text("x".into()),
        &opts(),
    );
    assert!(r.is_none());
}

#[test]
fn different_texts_produce_content_changed() {
    let r = compare_values(
        &CellValue::Text("a".into()),
        &CellValue::Text("b".into()),
        &opts(),
    )
    .unwrap();
    assert_eq!(r.reason, ValueDifferenceKind::ContentChanged);
}

#[test]
fn integer_vs_number_is_type_changed_by_default() {
    let r = compare_values(&CellValue::Integer(1), &CellValue::Number(1.0), &opts()).unwrap();
    assert_eq!(r.reason, ValueDifferenceKind::TypeChanged);
}

#[test]
fn integer_vs_number_equal_when_math_policy() {
    let mut o = opts();
    o.numeric_type = NumericTypePolicy::CompareMathematicalValue;
    let r = compare_values(&CellValue::Integer(1), &CellValue::Number(1.0), &o);
    assert!(r.is_none());
}

#[test]
fn text_vs_integer_is_type_changed() {
    let r = compare_values(
        &CellValue::Text("100".into()),
        &CellValue::Integer(100),
        &opts(),
    )
    .unwrap();
    assert_eq!(r.reason, ValueDifferenceKind::TypeChanged);
}

#[test]
fn equal_booleans_produce_no_change() {
    let r = compare_values(&CellValue::Bool(true), &CellValue::Bool(true), &opts());
    assert!(r.is_none());
}

#[test]
fn empty_vs_empty_produces_no_change() {
    let r = compare_values(&CellValue::Empty, &CellValue::Empty, &opts());
    assert!(r.is_none());
}

#[test]
fn formula_ignore_returns_none() {
    let r = compare_formulas(Some("=A1"), Some("=B1"), FormulaCompareMode::Ignore);
    assert!(r.is_none());
}

#[test]
fn equal_formulas_return_none() {
    let r = compare_formulas(Some("=A1+B1"), Some("=A1+B1"), FormulaCompareMode::RawText);
    assert!(r.is_none());
}

#[test]
fn different_formulas_return_change() {
    let r = compare_formulas(
        Some("=A1+B1"),
        Some("=A1+B1+C1"),
        FormulaCompareMode::RawText,
    )
    .unwrap();
    assert_eq!(r.old.as_ref().unwrap().raw, "=A1+B1");
    assert_eq!(r.new.as_ref().unwrap().raw, "=A1+B1+C1");
}

#[test]
fn formula_added() {
    let r = compare_formulas(None, Some("=SUM(A1:A10)"), FormulaCompareMode::RawText).unwrap();
    assert!(r.old.is_none());
    assert!(r.new.is_some());
}
