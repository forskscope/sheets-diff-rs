use super::*;
use calamine::Data;

#[test]
fn empty_maps_to_empty() {
    assert!(matches!(
        normalize_cell_value(&Data::Empty, false),
        CellValue::Empty
    ));
}

#[test]
fn string_maps_to_text() {
    let v = normalize_cell_value(&Data::String("hello".into()), false);
    assert!(matches!(v, CellValue::Text(s) if s == "hello"));
}

#[test]
fn int_maps_to_integer() {
    let v = normalize_cell_value(&Data::Int(42), false);
    assert!(matches!(v, CellValue::Integer(42)));
}

#[test]
fn float_maps_to_number() {
    let v = normalize_cell_value(&Data::Float(3.5), false);
    assert!(matches!(v, CellValue::Number(f) if (f - 3.5).abs() < 1e-12));
}

#[test]
fn bool_maps_to_bool() {
    assert!(matches!(
        normalize_cell_value(&Data::Bool(true), false),
        CellValue::Bool(true)
    ));
    assert!(matches!(
        normalize_cell_value(&Data::Bool(false), false),
        CellValue::Bool(false)
    ));
}

#[test]
fn error_div0_maps_correctly() {
    let v = normalize_cell_value(&Data::Error(CellErrorType::Div0), false);
    assert!(matches!(v, CellValue::Error(CellError::Div0)));
}

#[test]
fn error_ref_maps_correctly() {
    let v = normalize_cell_value(&Data::Error(CellErrorType::Ref), false);
    assert!(matches!(v, CellValue::Error(CellError::Ref)));
}

#[test]
fn datetime_iso_maps_with_iso_string() {
    let v = normalize_cell_value(&Data::DateTimeIso("2024-01-01T00:00:00".into()), false);
    match v {
        CellValue::DateTime(dt) => {
            assert_eq!(dt.iso.as_deref(), Some("2024-01-01T00:00:00"));
            // D-01: the serial is a placeholder, not a real Excel serial.
            assert!(!dt.has_serial);
        }
        other => panic!("expected DateTime, got {other:?}"),
    }
}

#[test]
fn duration_iso_maps_to_duration() {
    let v = normalize_cell_value(&Data::DurationIso("PT1H30M".into()), false);
    match v {
        CellValue::Duration(d) => {
            assert_eq!(d.iso.as_deref(), Some("PT1H30M"));
        }
        other => panic!("expected Duration, got {other:?}"),
    }
}

// D-02: `is_1904` threading ------------------------------------------------

#[test]
fn datetime_carries_workbook_is_1904_flag() {
    use calamine::{ExcelDateTime, ExcelDateTimeType};
    // `ExcelDateTime`'s own `is_1904` field is private (no public getter) —
    // this is `normalize_cell_value`'s `is_1904` *parameter*, threaded in
    // from the workbook-level `Xlsx::has_1904_epoch()`, that this test
    // exercises.
    let dt = ExcelDateTime::new(1.0, ExcelDateTimeType::DateTime, false);
    let v = normalize_cell_value(&Data::DateTime(dt), false);
    assert!(matches!(v, CellValue::DateTime(d) if !d.is_1904 && d.has_serial));

    let dt = ExcelDateTime::new(1.0, ExcelDateTimeType::DateTime, false);
    let v = normalize_cell_value(&Data::DateTime(dt), true);
    assert!(matches!(v, CellValue::DateTime(d) if d.is_1904 && d.has_serial));
}

// RFC-033 §4 equality policy tests ----------------------------------------

#[test]
fn integer_and_number_are_distinct_values() {
    let a = normalize_cell_value(&Data::Int(1), false);
    let b = normalize_cell_value(&Data::Float(1.0), false);
    assert_ne!(a, b);
}

#[test]
fn text_and_integer_are_distinct() {
    let a = normalize_cell_value(&Data::String("100".into()), false);
    let b = normalize_cell_value(&Data::Int(100), false);
    assert_ne!(a, b);
}
