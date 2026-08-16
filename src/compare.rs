//! Cell-level value and formula comparison (RFC-010, RFC-018, RFC-019).

use crate::model::{
    CellDateTime, CellValue, FormulaChange, FormulaText, ValueChange, ValueDifferenceKind,
};
use crate::options::{
    DateComparePolicy, FormulaCompareMode, NumberComparePolicy, NumericTypePolicy,
    TypeMismatchPolicy, ValueCompareOptions,
};

// ---------------------------------------------------------------------------
// Value comparison
// ---------------------------------------------------------------------------

/// Compare two `CellValue`s under the supplied options.
///
/// Returns `Some(ValueChange)` when the values differ; `None` when equal.
pub fn compare_values(
    old: &CellValue,
    new: &CellValue,
    opts: &ValueCompareOptions,
) -> Option<ValueChange> {
    use CellValue::*;

    let reason = match (old, new) {
        // Same variant comparisons
        (Empty, Empty) => return None,
        (Text(a), Text(b)) if a == b => return None,
        (Text(a), Text(b)) => {
            debug_assert!(a != b);
            ValueDifferenceKind::ContentChanged
        }
        (Bool(a), Bool(b)) if a == b => return None,
        (Bool(_), Bool(_)) => ValueDifferenceKind::ContentChanged,
        (Integer(a), Integer(b)) if a == b => return None,
        (Integer(_), Integer(_)) => ValueDifferenceKind::ContentChanged,
        (Number(a), Number(b)) => compare_floats(*a, *b, &opts.number)?,
        (Error(a), Error(b)) if a == b => return None,
        (Error(_), Error(_)) => ValueDifferenceKind::ErrorKindChanged,
        (DateTime(a), DateTime(b)) => {
            if datetime_equal(a, b) {
                return None;
            }
            match opts.date {
                DateComparePolicy::ExactRepresentation => ValueDifferenceKind::DateTimeChanged,
                DateComparePolicy::NormalizeEquivalentDateTimes => {
                    // Attempt serial normalization across 1900/1904 systems.
                    // Only meaningful when both sides carry a genuine serial
                    // (D-01) — an ISO-only value has no epoch to normalise.
                    if a.has_serial
                        && b.has_serial
                        && normalized_serial_eq(a.serial, a.is_1904, b.serial, b.is_1904)
                    {
                        return None;
                    }
                    ValueDifferenceKind::DateTimeChanged
                }
            }
        }
        (Duration(a), Duration(b)) => {
            let equal = match (&a.iso, &b.iso) {
                // Both carry an ISO string: it is the authoritative
                // representation for a duration (RFC-019 / D-01 — `serial`
                // is currently always a `0.0` placeholder here; comparing
                // `iso` is what actually distinguishes two durations).
                (Some(ai), Some(bi)) => ai == bi,
                (None, None) => a.serial == b.serial,
                // One side has an ISO string and the other doesn't: never
                // silently equal (D-01) — there is no reliable common
                // representation to compare through.
                _ => false,
            };
            if equal {
                return None;
            }
            ValueDifferenceKind::ContentChanged
        }
        (Unsupported { display: a, .. }, Unsupported { display: b, .. }) if a == b => return None,
        (Unsupported { .. }, Unsupported { .. }) => ValueDifferenceKind::ContentChanged,

        // Cross-type: Integer vs Number
        (Integer(i), Number(f)) | (Number(f), Integer(i)) => match opts.numeric_type {
            NumericTypePolicy::PreserveType => ValueDifferenceKind::TypeChanged,
            NumericTypePolicy::CompareMathematicalValue => {
                if *i as f64 == *f {
                    return None;
                }
                ValueDifferenceKind::ContentChanged
            }
        },

        // Cross-type: everything else
        _ => match opts.type_mismatch {
            TypeMismatchPolicy::Different => ValueDifferenceKind::TypeChanged,
            TypeMismatchPolicy::CompareDisplayString => {
                let a_str = old.display_string();
                let b_str = new.display_string();
                if a_str == b_str {
                    return None;
                }
                ValueDifferenceKind::DisplayStringChanged
            }
        },
    };

    Some(ValueChange {
        old: old.clone(),
        new: new.clone(),
        reason,
    })
}

/// Equality for the default (`ExactRepresentation`) date/time comparison.
///
/// D-01: a value from `Data::DateTimeIso` has no genuine Excel serial — its
/// `serial` field is a `0.0` placeholder (`has_serial: false`) and `iso` is
/// the only meaningful representation. A value from `Data::DateTime` always
/// has a genuine serial (`has_serial: true`); when the `chrono` feature is
/// enabled it may *also* carry a synthesized `iso` string, but that string
/// is redundant with the serial, not authoritative — comparing it instead
/// of the serial would risk losing precision (the synthesized string has
/// only second resolution) and would make the comparison result depend on
/// whether `chrono` is enabled, which must not happen.
///
/// So: two genuine serials compare via serial/`is_1904`/`kind`, unchanged
/// from before. Two ISO-only values compare via `iso`. A serial-based value
/// against an ISO-only value has no shared representation to compare
/// through and is never silently equal.
fn datetime_equal(a: &CellDateTime, b: &CellDateTime) -> bool {
    match (a.has_serial, b.has_serial) {
        (true, true) => a.serial == b.serial && a.is_1904 == b.is_1904 && a.kind == b.kind,
        (false, false) => a.iso == b.iso,
        _ => false,
    }
}

fn compare_floats(a: f64, b: f64, policy: &NumberComparePolicy) -> Option<ValueDifferenceKind> {
    let equal = match policy {
        NumberComparePolicy::Exact => a == b || (a.is_nan() && b.is_nan()),
        NumberComparePolicy::AbsoluteTolerance(tol) => (a - b).abs() <= *tol,
        NumberComparePolicy::RelativeTolerance(tol) => {
            let denom = a.abs().max(b.abs());
            denom == 0.0 || (a - b).abs() / denom <= *tol
        }
        NumberComparePolicy::AbsoluteOrRelative { abs, rel } => {
            let abs_ok = (a - b).abs() <= *abs;
            let denom = a.abs().max(b.abs());
            let rel_ok = denom == 0.0 || (a - b).abs() / denom <= *rel;
            abs_ok || rel_ok
        }
    };
    if equal {
        None
    } else {
        Some(ValueDifferenceKind::NumericOutsideTolerance)
    }
}

/// Normalise serials across 1900 / 1904 date systems.
/// The offset between the two systems is 1462 days.
fn normalized_serial_eq(a_serial: f64, a_1904: bool, b_serial: f64, b_1904: bool) -> bool {
    const OFFSET: f64 = 1462.0;
    let a_norm = if a_1904 { a_serial + OFFSET } else { a_serial };
    let b_norm = if b_1904 { b_serial + OFFSET } else { b_serial };
    a_norm == b_norm
}

// ---------------------------------------------------------------------------
// Formula comparison (RFC-018)
// ---------------------------------------------------------------------------

/// Compare formula strings under the configured mode.
///
/// Returns `Some(FormulaChange)` when the formulas differ; `None` when equal or
/// when the mode is `Ignore`.
pub fn compare_formulas(
    old_formula: Option<&str>,
    new_formula: Option<&str>,
    mode: FormulaCompareMode,
) -> Option<FormulaChange> {
    if mode == FormulaCompareMode::Ignore {
        return None;
    }

    let old_text = old_formula.map(|r| FormulaText {
        raw: r.to_owned(),
        normalized: None, // NormalizedText mode guard is in options validation
    });
    let new_text = new_formula.map(|r| FormulaText {
        raw: r.to_owned(),
        normalized: None,
    });

    // Equal?
    let old_raw = old_formula.unwrap_or("");
    let new_raw = new_formula.unwrap_or("");
    if old_raw == new_raw {
        return None;
    }

    Some(FormulaChange {
        old: old_text,
        new: new_text,
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
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
        assert!(
            compare_values(&CellValue::DateTime(a), &CellValue::DateTime(b), &opts()).is_none()
        );
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
}
