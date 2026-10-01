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
        normalized: None, // no formula normaliser exists (RFC-018)
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
mod tests;
