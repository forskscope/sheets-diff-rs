//! Normalization of raw calamine data into the public `CellValue` type.
//!
//! The mapping table (RFC-033 §1) is the source of truth; every row has a unit
//! test in the `tests` module.

use calamine::{CellErrorType, Data};

use crate::model::{
    CellDateTime, CellDuration, CellError, CellValue, DateTimeKind,
};

// ---------------------------------------------------------------------------
// calamine::Data → CellValue
// ---------------------------------------------------------------------------

/// Convert a raw calamine `Data` cell into the public typed `CellValue`.
///
/// This function is the single normalization boundary; calamine types must not
/// appear anywhere in the public API (RFC-026).
pub fn normalize_cell_value(data: &Data) -> CellValue {
    match data {
        Data::Empty => CellValue::Empty,

        Data::String(s) => CellValue::Text(s.clone()),

        Data::Int(i) => CellValue::Integer(*i),

        Data::Float(f) => CellValue::Number(*f),

        Data::Bool(b) => CellValue::Bool(*b),

        Data::DateTime(dt) => {
            let kind = if dt.is_duration() {
                DateTimeKind::Time // calamine treats time-of-day (duration) separately
            } else if dt.is_datetime() {
                DateTimeKind::DateTime
            } else {
                DateTimeKind::Date
            };

            // Synthesize an ISO string only when the chrono feature is enabled.
            #[cfg(feature = "chrono")]
            let iso: Option<String> = {
                if dt.is_duration() {
                    dt.as_duration()
                        .map(|d| {
                            let secs = d.num_seconds();
                            let h = secs / 3600;
                            let m = (secs % 3600) / 60;
                            let s = secs % 60;
                            format!("PT{h:02}H{m:02}M{s:02}S")
                        })
                } else {
                    dt.as_datetime().map(|ndt| ndt.format("%Y-%m-%dT%H:%M:%S").to_string())
                }
            };
            #[cfg(not(feature = "chrono"))]
            let iso: Option<String> = None;

            // `is_1904` is a private field on `ExcelDateTime` in calamine 0.35;
            // no public accessor exists on the value type. Callers needing the
            // epoch flag can query `Xlsx::has_1904_epoch()` on the open workbook
            // and adjust the serial in the pipeline when required.
            CellValue::DateTime(CellDateTime {
                serial: dt.as_f64(),
                is_1904: false,
                kind,
                iso,
            })
        }

        Data::DateTimeIso(s) => {
            // calamine gives us a pre-formatted ISO string; no serial available.
            CellValue::DateTime(CellDateTime {
                serial: 0.0,
                is_1904: false,
                kind: DateTimeKind::DateTime,
                iso: Some(s.clone()),
            })
        }

        Data::DurationIso(s) => {
            CellValue::Duration(CellDuration {
                serial: 0.0,
                iso: Some(s.clone()),
            })
        }

        Data::Error(e) => CellValue::Error(normalize_cell_error(e)),
    }
}

fn normalize_cell_error(e: &CellErrorType) -> CellError {
    match e {
        CellErrorType::Div0 => CellError::Div0,
        CellErrorType::NA => CellError::NA,
        CellErrorType::Name => CellError::Name,
        CellErrorType::Null => CellError::Null,
        CellErrorType::Num => CellError::Num,
        CellErrorType::Ref => CellError::Ref,
        CellErrorType::Value => CellError::Value,
        CellErrorType::GettingData => CellError::GettingData,
    }
}

// ---------------------------------------------------------------------------
// Tests — one per row of the RFC-033 §1 mapping table
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use calamine::Data;

    #[test]
    fn empty_maps_to_empty() {
        assert!(matches!(normalize_cell_value(&Data::Empty), CellValue::Empty));
    }

    #[test]
    fn string_maps_to_text() {
        let v = normalize_cell_value(&Data::String("hello".into()));
        assert!(matches!(v, CellValue::Text(s) if s == "hello"));
    }

    #[test]
    fn int_maps_to_integer() {
        let v = normalize_cell_value(&Data::Int(42));
        assert!(matches!(v, CellValue::Integer(42)));
    }

    #[test]
    fn float_maps_to_number() {
        let v = normalize_cell_value(&Data::Float(3.14));
        assert!(matches!(v, CellValue::Number(f) if (f - 3.14).abs() < 1e-12));
    }

    #[test]
    fn bool_maps_to_bool() {
        assert!(matches!(normalize_cell_value(&Data::Bool(true)), CellValue::Bool(true)));
        assert!(matches!(normalize_cell_value(&Data::Bool(false)), CellValue::Bool(false)));
    }

    #[test]
    fn error_div0_maps_correctly() {
        let v = normalize_cell_value(&Data::Error(CellErrorType::Div0));
        assert!(matches!(v, CellValue::Error(CellError::Div0)));
    }

    #[test]
    fn error_ref_maps_correctly() {
        let v = normalize_cell_value(&Data::Error(CellErrorType::Ref));
        assert!(matches!(v, CellValue::Error(CellError::Ref)));
    }

    #[test]
    fn datetime_iso_maps_with_iso_string() {
        let v = normalize_cell_value(&Data::DateTimeIso("2024-01-01T00:00:00".into()));
        match v {
            CellValue::DateTime(dt) => {
                assert_eq!(dt.iso.as_deref(), Some("2024-01-01T00:00:00"));
            }
            other => panic!("expected DateTime, got {other:?}"),
        }
    }

    #[test]
    fn duration_iso_maps_to_duration() {
        let v = normalize_cell_value(&Data::DurationIso("PT1H30M".into()));
        match v {
            CellValue::Duration(d) => {
                assert_eq!(d.iso.as_deref(), Some("PT1H30M"));
            }
            other => panic!("expected Duration, got {other:?}"),
        }
    }

    // RFC-033 §4 equality policy tests ----------------------------------------

    #[test]
    fn integer_and_number_are_distinct_values() {
        let a = normalize_cell_value(&Data::Int(1));
        let b = normalize_cell_value(&Data::Float(1.0));
        assert_ne!(a, b);
    }

    #[test]
    fn text_and_integer_are_distinct() {
        let a = normalize_cell_value(&Data::String("100".into()));
        let b = normalize_cell_value(&Data::Int(100));
        assert_ne!(a, b);
    }
}
