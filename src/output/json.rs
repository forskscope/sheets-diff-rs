//! JSON output helpers (RFC-014).
//!
//! Available only when the `serde` feature is enabled.
//! All public types in `model` derive `Serialize`; this module adds the
//! convenience `to_json` / `to_json_pretty` wrappers.

use crate::model::WorkbookDiff;

/// Serialise a `WorkbookDiff` to a compact JSON string.
///
/// Returns an error string if serialisation fails (should not happen for
/// well-formed results, but the caller should handle the `Result`).
pub fn to_json(diff: &WorkbookDiff) -> Result<String, String> {
    serde_json::to_string(diff).map_err(|e| e.to_string())
}

/// Serialise a `WorkbookDiff` to a pretty-printed JSON string.
pub fn to_json_pretty(diff: &WorkbookDiff) -> Result<String, String> {
    serde_json::to_string_pretty(diff).map_err(|e| e.to_string())
}
