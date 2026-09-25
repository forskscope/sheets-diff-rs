//! JSON output helpers (RFC-014).
//!
//! Available only when the `serde` feature is enabled.
//! All public types in `model` derive `Serialize`; this module adds the
//! convenience `to_json` / `to_json_pretty` wrappers.

use crate::model::WorkbookDiff;

/// Serialise a `WorkbookDiff` to a compact JSON string.
///
/// **This returns a `String`, not a `Result`, because it cannot fail** — and you can check why:
///
/// * the serialised shape contains **no maps or sets**, so `serde_json`'s one structural failure, a non-string map
///   key, cannot arise;
/// * **every `Serialize` in the model is derived** — no hand-written impl exists that could return an error;
/// * **non-finite floats serialise as `null`**, they do not fail (`CellValue::Number(f64)` is the only float in the
///   model);
/// * the output is built in memory, so there is no I/O to fail.
///
/// **If a future field breaks any of those — a `HashMap`, a custom `Serialize`, a type that can refuse to serialise —
/// this function needs its `Result` back**, and that is a breaking change. Adding such a field is therefore a
/// decision to make on purpose; `tests/json_infallible.rs` and the derive-only scan recorded in the M10 unit 09
/// review request are what to re-run.
pub fn to_json(diff: &WorkbookDiff) -> String {
    serde_json::to_string(diff)
        .expect("the serialised shape has no maps and only derived impls; see the docs above")
}

/// Serialise a `WorkbookDiff` to a pretty-printed JSON string. The same output as [`to_json`], indented, and
/// infallible for the same reasons (see there).
pub fn to_json_pretty(diff: &WorkbookDiff) -> String {
    serde_json::to_string_pretty(diff)
        .expect("the serialised shape has no maps and only derived impls; see [`to_json`]")
}
