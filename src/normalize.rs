//! Normalization of raw calamine data into the public `CellValue` type.
//!
//! The mapping table (RFC-033 §1) is the source of truth; every row has a unit
//! test in the `tests` module.

use calamine::{CellErrorType, Data};

use crate::model::{CellDateTime, CellDuration, CellError, CellValue, DateTimeKind};

// ---------------------------------------------------------------------------
// calamine::Data → CellValue
// ---------------------------------------------------------------------------

/// Convert a raw calamine `Data` cell into the public typed `CellValue`.
///
/// `is_1904` is the workbook-level date epoch flag
/// (`Xlsx::has_1904_epoch()`), read once per workbook and threaded in here
/// rather than re-derived per cell (RFC-019 / D-02).
///
/// This function is the single normalization boundary; calamine types must not
/// appear anywhere in the public API (RFC-026).
pub fn normalize_cell_value(data: &Data, is_1904: bool) -> CellValue {
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
                    dt.as_duration().map(|d| {
                        let secs = d.num_seconds();
                        let h = secs / 3600;
                        let m = (secs % 3600) / 60;
                        let s = secs % 60;
                        format!("PT{h:02}H{m:02}M{s:02}S")
                    })
                } else {
                    dt.as_datetime()
                        .map(|ndt| ndt.format("%Y-%m-%dT%H:%M:%S").to_string())
                }
            };
            #[cfg(not(feature = "chrono"))]
            let iso: Option<String> = None;

            // `ExcelDateTime` has no public `is_1904` accessor; the epoch flag
            // is workbook-level, read once via `Xlsx::has_1904_epoch()` and
            // passed in as `is_1904` (RFC-019 / D-02).
            CellValue::DateTime(CellDateTime {
                serial: dt.as_f64(),
                is_1904,
                kind,
                iso,
                has_serial: true,
            })
        }

        Data::DateTimeIso(s) => {
            // calamine gives us a pre-formatted ISO string; no serial available.
            // `has_serial: false` — RFC-019 / D-01: `serial` is a placeholder,
            // not a real Excel serial, and comparison must not treat it as one.
            CellValue::DateTime(CellDateTime {
                serial: 0.0,
                is_1904,
                kind: DateTimeKind::DateTime,
                iso: Some(s.clone()),
                has_serial: false,
            })
        }

        Data::DurationIso(s) => CellValue::Duration(CellDuration {
            serial: 0.0,
            iso: Some(s.clone()),
        }),

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
mod tests;
