//! Cell-level value and formula comparison (RFC-010, RFC-018, RFC-019).

use crate::model::{
    CellValue, FormulaChange, FormulaText, ValueChange, ValueDifferenceKind,
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
        (Number(a), Number(b)) => match compare_floats(*a, *b, &opts.number) {
            Some(r) => r,
            None => return None,
        },
        (Error(a), Error(b)) if a == b => return None,
        (Error(_), Error(_)) => ValueDifferenceKind::ErrorKindChanged,
        (DateTime(a), DateTime(b)) => {
            if a.serial == b.serial && a.is_1904 == b.is_1904 && a.kind == b.kind {
                return None;
            }
            match opts.date {
                DateComparePolicy::ExactRepresentation => ValueDifferenceKind::DateTimeChanged,
                DateComparePolicy::NormalizeEquivalentDateTimes => {
                    // Attempt serial normalization across 1900/1904 systems.
                    if normalized_serial_eq(a.serial, a.is_1904, b.serial, b.is_1904) {
                        return None;
                    }
                    ValueDifferenceKind::DateTimeChanged
                }
            }
        }
        (Duration(a), Duration(b)) => {
            if a.serial == b.serial {
                return None;
            }
            ValueDifferenceKind::ContentChanged
        }
        (Unsupported { display: a, .. }, Unsupported { display: b, .. }) if a == b => {
            return None
        }
        (Unsupported { .. }, Unsupported { .. }) => ValueDifferenceKind::ContentChanged,

        // Cross-type: Integer vs Number
        (Integer(i), Number(f)) | (Number(f), Integer(i)) => {
            match opts.numeric_type {
                NumericTypePolicy::PreserveType => ValueDifferenceKind::TypeChanged,
                NumericTypePolicy::CompareMathematicalValue => {
                    if *i as f64 == *f {
                        return None;
                    }
                    ValueDifferenceKind::ContentChanged
                }
            }
        }

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

    Some(ValueChange { old: old.clone(), new: new.clone(), reason })
}

fn compare_floats(
    a: f64,
    b: f64,
    policy: &NumberComparePolicy,
) -> Option<ValueDifferenceKind> {
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

    Some(FormulaChange { old: old_text, new: new_text })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::CellValue;
    use crate::options::ValueCompareOptions;

    fn opts() -> ValueCompareOptions {
        ValueCompareOptions::default()
    }

    #[test]
    fn equal_texts_produce_no_change() {
        let r = compare_values(&CellValue::Text("x".into()), &CellValue::Text("x".into()), &opts());
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
        let r = compare_formulas(Some("=A1+B1"), Some("=A1+B1+C1"), FormulaCompareMode::RawText)
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
