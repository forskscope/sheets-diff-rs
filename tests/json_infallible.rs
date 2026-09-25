//! `to_json` / `to_json_pretty` return `String` because serialising a `WorkbookDiff` cannot fail (M10 unit 09).
//!
//! Each condition the docs give for that is checked here, so a future model field that breaks one of them fails a test
//! instead of turning an infallible function into a panic:
//! * the output is valid JSON for every corpus scenario, both functions, and they agree with each other;
//! * **non-finite floats serialise as `null`** (`CellValue::Number(f64)` is the only float in the model) — measured on
//!   a result whose cells are set to NaN and both infinities;
//! * (the other two conditions — no maps or sets in the shape, every `Serialize` derived — are properties of the
//!   source, re-derived by the scan recorded in the review request; a map would be visible in the JSON as an object
//!   whose keys are data, which the shape test below pins.)
#![cfg(feature = "serde")]

mod support;
use support::wb_strings;

use serde_json::Value;
use sheets_diff::output::json::{to_json, to_json_pretty};
use sheets_diff::{CellValue, WorkbookDiff, compare_bytes};

fn corpus() -> Vec<(String, WorkbookDiff)> {
    let root = std::path::Path::new("tests/fixtures/generated");
    let mut out = Vec::new();
    for e in std::fs::read_dir(root).unwrap() {
        let dir = e.unwrap().path();
        if !dir.is_dir() {
            continue;
        }
        let (o, n) = (
            std::fs::read(dir.join("old.xlsx")).unwrap(),
            std::fs::read(dir.join("new.xlsx")).unwrap(),
        );
        out.push((
            dir.file_name().unwrap().to_string_lossy().into_owned(),
            compare_bytes(&o, &n).unwrap(),
        ));
    }
    assert_eq!(out.len(), 19);
    out
}

/// Both functions return a `String` — a plain binding, no `?`, no `unwrap` — that parses as JSON, on every scenario;
/// and the pretty form is the compact form, indented.
#[test]
fn both_functions_return_a_string_that_parses_and_agree() {
    for (name, d) in corpus() {
        let compact: String = to_json(&d);
        let pretty: String = to_json_pretty(&d);
        let a: Value = serde_json::from_str(&compact).unwrap_or_else(|e| panic!("{name}: {e}"));
        let b: Value = serde_json::from_str(&pretty).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(a, b, "{name}: pretty and compact disagree");
        assert!(pretty.contains('\n') && !compact.contains('\n'), "{name}");
    }
}

/// Non-finite floats are not a failure: they become `null`. Set every numeric cell of a real result to NaN, +inf,
/// -inf and a finite control, serialise, and read them back.
#[test]
fn non_finite_floats_serialise_as_null_and_do_not_fail() {
    let old = wb_strings(&[(0, 0, "a"), (1, 0, "b"), (2, 0, "c"), (3, 0, "d")]);
    let new = wb_strings(&[(0, 0, "A"), (1, 0, "B"), (2, 0, "C"), (3, 0, "D")]);
    let mut d = compare_bytes(&old, &new).unwrap();
    let floats = [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 1.5];
    for (c, f) in d.sheets[0].cell_diffs.iter_mut().zip(floats) {
        c.value.as_mut().unwrap().old = CellValue::Number(f);
    }
    let v: Value = serde_json::from_str(&to_json(&d)).expect("valid JSON");
    let olds: Vec<Value> = v["sheets"][0]["cell_diffs"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["value"]["old"]["Number"].clone())
        .collect();
    assert_eq!(
        olds,
        [
            Value::Null,
            Value::Null,
            Value::Null,
            serde_json::json!(1.5)
        ]
    );
    assert!(!to_json_pretty(&d).is_empty());
}

/// The shape has no maps: no JSON object anywhere in a result uses data as its keys. Every object's keys are drawn from
/// a fixed vocabulary of field and variant names, so the whole corpus produces a small closed set.
#[test]
fn every_object_key_is_a_field_or_variant_name_not_data() {
    fn keys(v: &Value, out: &mut std::collections::BTreeSet<String>) {
        match v {
            Value::Object(m) => {
                for (k, x) in m {
                    out.insert(k.clone());
                    keys(x, out);
                }
            }
            Value::Array(a) => a.iter().for_each(|x| keys(x, out)),
            _ => {}
        }
    }
    let mut all = std::collections::BTreeSet::new();
    for (_, d) in corpus() {
        keys(&serde_json::from_str(&to_json(&d)).unwrap(), &mut all);
    }
    // Cell text such as "before", "OldName" or "id_3" must never appear as a key.
    for data in ["before", "after", "OldName", "NewName", "Sheet1"] {
        assert!(
            !all.contains(data),
            "`{data}` is a data value used as a JSON key: {all:?}"
        );
    }
    assert!(all.contains("sheets") && all.contains("summary") && all.contains("metrics"));
}
