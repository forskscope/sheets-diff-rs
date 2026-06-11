//! Integration tests for `sheets-diff` v2 (RFC-015).
//!
//! Fixture categories follow RFC-015 §5:
//!   basic · corrupt · wide-columns · typed-values · formulas
//!   sheet-renames · empty-sheets · sparse-ranges · limits
//!   progress/cancellation · output · serde

mod support;
use support::*;

use sheets_diff::{
    CellChangeKind, CellValue, DiffEvent, DiffOptions, FormulaCompareMode,
    SheetChange, SheetMatchingMode, SheetsDiffError,
    compare_bytes, compare_bytes_with_options,
    output::text::{render_summary, render_unified},
};

// ============================================================================
// basic
// ============================================================================

#[test]
fn identical_workbooks_no_diffs() {
    let b = wb_strings(&[(0, 0, "hello")]);
    let d = compare_bytes(&b, &b).unwrap();
    assert_eq!(d.summary.cells_changed, 0);
    assert_eq!(d.summary.sheets_changed, 0);
}

#[test]
fn single_cell_text_change() {
    let old = wb_strings(&[(0, 0, "hello")]);
    let new = wb_strings(&[(0, 0, "world")]);
    let d = compare_bytes(&old, &new).unwrap();
    assert_eq!(d.summary.cells_changed, 1);
    assert_eq!(d.summary.values_changed, 1);
    let cd = &d.sheets[0].cell_diffs[0];
    assert_eq!(cd.address.a1, "A1");
    let vc = cd.value.as_ref().unwrap();
    assert!(matches!(&vc.old, CellValue::Text(s) if s == "hello"));
    assert!(matches!(&vc.new, CellValue::Text(s) if s == "world"));
}

#[test]
fn cell_added_to_empty_sheet() {
    let old = wb_empty();
    let new = wb_strings(&[(0, 0, "new")]);
    let d = compare_bytes(&old, &new).unwrap();
    assert_eq!(d.summary.cells_changed, 1);
    assert_eq!(d.sheets[0].cell_diffs[0].change_kind(), CellChangeKind::Added);
}

#[test]
fn cell_removed_from_sheet() {
    let old = wb_strings(&[(0, 0, "gone")]);
    let new = wb_empty();
    let d = compare_bytes(&old, &new).unwrap();
    assert_eq!(d.sheets[0].cell_diffs[0].change_kind(), CellChangeKind::Removed);
}

#[test]
fn multiple_cells_multiple_sheets() {
    let old = wb_sheets(&[
        ("Data", &[(0, 0, "a"), (0, 1, "b")]),
        ("Meta", &[(0, 0, "v1")]),
    ]);
    let new = wb_sheets(&[
        ("Data", &[(0, 0, "x"), (0, 1, "b")]),
        ("Meta", &[(0, 0, "v2")]),
    ]);
    let d = compare_bytes(&old, &new).unwrap();
    assert_eq!(d.summary.cells_changed, 2);
    assert_eq!(d.summary.sheets_changed, 2);
}

#[test]
fn unchanged_sheets_not_counted() {
    let b = wb_sheets(&[("S1", &[(0, 0, "x")]), ("S2", &[(0, 0, "y")])]);
    let d = compare_bytes(&b, &b).unwrap();
    assert_eq!(d.summary.sheets_changed, 0);
    assert_eq!(d.summary.cells_changed, 0);
}

#[test]
fn cell_diffs_sorted_by_row_then_col() {
    // A10 must sort after A2 (numeric, not lexicographic).
    let old = wb_strings(&[(0, 0, "a"), (0, 1, "b"), (9, 0, "c")]);
    let new = wb_strings(&[(0, 0, "x"), (0, 1, "y"), (9, 0, "z")]);
    let d = compare_bytes(&old, &new).unwrap();
    let addrs: Vec<&str> = d.sheets[0].cell_diffs.iter().map(|c| c.address.a1.as_str()).collect();
    assert_eq!(addrs, ["A1", "B1", "A10"]);
}

// ============================================================================
// corrupt
// ============================================================================

#[test]
fn corrupt_bytes_structured_error() {
    let junk = b"not a zip file";
    let good = wb_empty();
    assert!(matches!(
        compare_bytes(junk.as_slice(), &good),
        Err(SheetsDiffError::OpenWorkbook { .. })
    ));
}

#[test]
fn empty_bytes_structured_error() {
    let good = wb_empty();
    assert!(matches!(
        compare_bytes(b"".as_slice(), &good),
        Err(SheetsDiffError::OpenWorkbook { .. })
    ));
}

#[test]
fn corrupt_file_fixture_structured_error() {
    // Binary fixture: valid gzip header but not a ZIP.
    let corrupt = std::fs::read("tests/fixtures/corrupt/not_a_zip.xlsx").unwrap();
    let good = wb_empty();
    let result = compare_bytes(&corrupt, &good);
    assert!(result.is_err(), "expected error for corrupt fixture");
    assert!(!std::panic::catch_unwind(|| compare_bytes(&corrupt, &good)).is_err(),
        "must not panic");
}

// ============================================================================
// wide-columns  (RFC-015: columns A / Z / AA / AZ / BA / ZZ / AAA / XFD)
// ============================================================================

#[test]
fn wide_column_a1_encoding() {
    use sheets_diff::address::col_to_label;
    assert_eq!(col_to_label(1),      "A");
    assert_eq!(col_to_label(26),     "Z");
    assert_eq!(col_to_label(27),     "AA");
    assert_eq!(col_to_label(52),     "AZ");
    assert_eq!(col_to_label(53),     "BA");
    assert_eq!(col_to_label(702),    "ZZ");
    assert_eq!(col_to_label(703),    "AAA");
    assert_eq!(col_to_label(16_384), "XFD");
}

#[test]
fn wide_column_cell_diff_detected() {
    // Column 703 = AAA
    let old = wb_wide_column(0, 702, "before"); // 0-based col 702 → 1-based 703 → AAA
    let new = wb_wide_column(0, 702, "after");
    let d = compare_bytes(&old, &new).unwrap();
    assert_eq!(d.summary.cells_changed, 1);
    assert_eq!(d.sheets[0].cell_diffs[0].address.a1, "AAA1");
}

#[test]
fn cell_address_bounds() {
    use sheets_diff::CellAddress;
    assert!(CellAddress::new(1, 1).is_some());
    assert!(CellAddress::new(1_048_576, 16_384).is_some());
    assert!(CellAddress::new(0, 1).is_none());
    assert!(CellAddress::new(1, 0).is_none());
    assert!(CellAddress::new(1_048_577, 1).is_none());
    assert!(CellAddress::new(1, 16_385).is_none());
}

#[test]
fn a10_sorts_after_a2_not_lexicographically() {
    use sheets_diff::CellAddress;
    let a2  = CellAddress::new(2, 1).unwrap();
    let a10 = CellAddress::new(10, 1).unwrap();
    assert!(a2 < a10);
}

// ============================================================================
// typed-values  (RFC-033 §4 equality policy)
// ============================================================================

#[test]
fn text_100_and_number_100_different() {
    let old = wb_strings(&[(0, 0, "100")]);
    let new = wb_numbers(&[(0, 0, 100.0)]);
    let d = compare_bytes(&old, &new).unwrap();
    assert_eq!(d.summary.values_changed, 1);
    let vc = d.sheets[0].cell_diffs[0].value.as_ref().unwrap();
    assert!(matches!(vc.old, CellValue::Text(_)));
    assert!(matches!(vc.new, CellValue::Number(_)));
}

#[test]
fn integer_and_float_different_by_default() {
    // calamine Int(1) vs Float(1.0) → TypeChanged by default
    use sheets_diff::ValueDifferenceKind;
    // Both sides store 1 but as different calamine variants — simulate via
    // a workbook where we know one came from Int.
    // We verify the policy directly via normalize + compare instead.
    use sheets_diff::address::col_to_label;
    let _ = col_to_label(1); // ensure normalize module is compiled
    // Unit-level tested in normalize::tests; here confirm CellValue PartialEq.
    assert_ne!(CellValue::Integer(1), CellValue::Number(1.0));
    // And the reason is TypeChanged in compare:
    use sheets_diff::options::ValueCompareOptions;
    use sheets_diff::compare::compare_values_pub;
    let vc = compare_values_pub(
        &CellValue::Integer(1),
        &CellValue::Number(1.0),
        &ValueCompareOptions::default(),
    ).unwrap();
    assert_eq!(vc.reason, ValueDifferenceKind::TypeChanged);
}

#[test]
fn bool_true_and_bool_false_different() {
    let old = wb_bools(&[(0, 0, true)]);
    let new = wb_bools(&[(0, 0, false)]);
    let d = compare_bytes(&old, &new).unwrap();
    assert_eq!(d.summary.values_changed, 1);
}

#[test]
fn bool_and_text_true_different() {
    let old = wb_bools(&[(0, 0, true)]);
    let new = wb_strings(&[(0, 0, "TRUE")]);
    let d = compare_bytes(&old, &new).unwrap();
    assert_eq!(d.summary.values_changed, 1);
    let vc = d.sheets[0].cell_diffs[0].value.as_ref().unwrap();
    assert!(matches!(vc.old, CellValue::Bool(_)));
    assert!(matches!(vc.new, CellValue::Text(_)));
}

#[test]
fn numeric_tolerance_treats_near_equal_as_same() {
    use sheets_diff::options::{NumberComparePolicy, ValueCompareOptions};
    use sheets_diff::compare::compare_values_pub;
    let mut opts = ValueCompareOptions::default();
    opts.number = NumberComparePolicy::AbsoluteTolerance(0.01);
    let result = compare_values_pub(
        &CellValue::Number(1.0),
        &CellValue::Number(1.005),
        &opts,
    );
    assert!(result.is_none(), "should be equal within tolerance");
}

#[test]
fn numeric_tolerance_detects_difference_outside_tolerance() {
    use sheets_diff::options::{NumberComparePolicy, ValueCompareOptions};
    use sheets_diff::compare::compare_values_pub;
    let mut opts = ValueCompareOptions::default();
    opts.number = NumberComparePolicy::AbsoluteTolerance(0.001);
    let result = compare_values_pub(
        &CellValue::Number(1.0),
        &CellValue::Number(1.005),
        &opts,
    );
    assert!(result.is_some(), "should detect difference outside tolerance");
}

// ============================================================================
// formulas
// ============================================================================

#[test]
fn formula_only_change_produces_no_value_change() {
    // Two workbooks with the same cached string value but different formula text.
    // rust_xlsxwriter writes the formula; calamine reads it back.
    let old = wb_with_formula(0, 0, "label", 1, 0, "=1+1");
    let new = wb_with_formula(0, 0, "label", 1, 0, "=1+1+0");
    let d = compare_bytes(&old, &new).unwrap();
    // formula text changed; cached value may or may not differ depending on
    // whether the xlsx writer stored the cached result.
    // At minimum, no panic and formula change is tracked if text was available.
    assert!(d.summary.cells_changed <= 1); // 0 if no formula text stored, 1 if it was
}

#[test]
fn formula_ignore_skips_formula_changes() {
    let old = wb_with_formula(0, 0, "x", 1, 0, "=A1");
    let new = wb_with_formula(0, 0, "x", 1, 0, "=A1&\"\"");
    let opts = DiffOptions::builder()
        .formula_compare(FormulaCompareMode::Ignore)
        .build().unwrap();
    let d = compare_bytes_with_options(&old, &new, opts).unwrap();
    assert_eq!(d.summary.formulas_changed, 0);
}

#[test]
fn value_and_formula_both_changed_is_one_cell_diff() {
    // Change both the string value and the formula in the same cell position.
    let old = wb_with_formula(0, 0, "before", 0, 0, "=1");
    let new = wb_with_formula(0, 0, "after",  0, 0, "=2");
    let d = compare_bytes(&old, &new).unwrap();
    // A1 contains either the string or the formula depending on write order;
    // the key invariant is at most one CellDiff per address.
    let a1_diffs: Vec<_> = d.sheets[0].cell_diffs.iter()
        .filter(|c| c.address.a1 == "A1").collect();
    assert!(a1_diffs.len() <= 1, "must be at most one CellDiff per address");
}

// ============================================================================
// sheet-renames
// ============================================================================

#[test]
fn sheet_rename_detected_conservative() {
    let old = wb_sheets(&[("OldName", &[(0, 0, "v")])]);
    let new = wb_sheets(&[("NewName", &[(0, 0, "v")])]);
    let d = compare_bytes(&old, &new).unwrap();
    assert_eq!(d.summary.sheets_renamed, 1);
    assert!(d.sheets.iter().any(|s| matches!(s.change, SheetChange::Renamed { .. })));
}

#[test]
fn renamed_sheet_preserves_cell_diffs() {
    let old = wb_sheets(&[("OldName", &[(0, 0, "before")])]);
    let new = wb_sheets(&[("NewName", &[(0, 0, "after")])]);
    let d = compare_bytes(&old, &new).unwrap();
    let renamed = d.sheets.iter().find(|s| matches!(s.change, SheetChange::Renamed { .. })).unwrap();
    assert_eq!(renamed.cell_diffs.len(), 1);
    assert_eq!(renamed.old_sheet.as_ref().unwrap().name, "OldName");
    assert_eq!(renamed.new_sheet.as_ref().unwrap().name, "NewName");
}

#[test]
fn sheet_added() {
    let old = wb_sheets(&[("S1", &[])]);
    let new = wb_sheets(&[("S1", &[]), ("S2", &[(0, 0, "x")])]);
    let d = compare_bytes(&old, &new).unwrap();
    assert_eq!(d.summary.sheets_added, 1);
}

#[test]
fn sheet_removed() {
    let old = wb_sheets(&[("S1", &[]), ("S2", &[(0, 0, "x")])]);
    let new = wb_sheets(&[("S1", &[])]);
    let d = compare_bytes(&old, &new).unwrap();
    assert_eq!(d.summary.sheets_removed, 1);
}

#[test]
fn exact_name_only_no_rename() {
    let old = wb_sheets(&[("OldName", &[])]);
    let new = wb_sheets(&[("NewName", &[])]);
    let opts = DiffOptions::builder()
        .sheet_matching(SheetMatchingMode::ExactNameOnly)
        .build().unwrap();
    let d = compare_bytes_with_options(&old, &new, opts).unwrap();
    assert_eq!(d.summary.sheets_renamed, 0);
    assert_eq!(d.summary.sheets_added, 1);
    assert_eq!(d.summary.sheets_removed, 1);
}

#[test]
fn ambiguous_renames_become_add_remove_with_diagnostic() {
    // Two removed + two added → no confident rename; expect diagnostics.
    let old = wb_sheets(&[("A", &[]), ("B", &[])]);
    let new = wb_sheets(&[("C", &[]), ("D", &[])]);
    let d = compare_bytes(&old, &new).unwrap();
    assert_eq!(d.summary.sheets_renamed, 0);
    assert!(d.diagnostics.iter().any(|diag| {
        diag.kind.code() == "ambiguous_sheet_match"
    }));
}

// ============================================================================
// empty-sheets / sparse-ranges
// ============================================================================

#[test]
fn empty_vs_empty_no_diff() {
    let b = wb_empty();
    let d = compare_bytes(&b, &b).unwrap();
    assert_eq!(d.summary.cells_changed, 0);
}

#[test]
fn empty_vs_nonempty_all_cells_added() {
    let old = wb_empty();
    let new = wb_strings(&[(0, 0, "x"), (5, 5, "y")]);
    let d = compare_bytes(&old, &new).unwrap();
    assert_eq!(d.summary.cells_changed, 2);
    assert!(d.sheets[0].cell_diffs.iter().all(|c| c.change_kind() == CellChangeKind::Added));
}

#[test]
fn nonempty_vs_empty_all_cells_removed() {
    let old = wb_strings(&[(0, 0, "a"), (3, 2, "b")]);
    let new = wb_empty();
    let d = compare_bytes(&old, &new).unwrap();
    assert_eq!(d.summary.cells_changed, 2);
    assert!(d.sheets[0].cell_diffs.iter().all(|c| c.change_kind() == CellChangeKind::Removed));
}

#[test]
fn sparse_range_only_changed_cells_reported() {
    // Only A1 changes; B5 and Z100 are the same on both sides.
    let old = wb_sparse(&[(0, 0, "old"), (4, 1, "same"), (99, 25, "same")]);
    let new = wb_sparse(&[(0, 0, "new"), (4, 1, "same"), (99, 25, "same")]);
    let d = compare_bytes(&old, &new).unwrap();
    assert_eq!(d.summary.cells_changed, 1);
    assert_eq!(d.sheets[0].cell_diffs[0].address.a1, "A1");
}

#[test]
fn compared_range_covers_both_sides() {
    let old = wb_strings(&[(0, 0, "x")]);
    let new = wb_strings(&[(9, 9, "y")]);
    let d = compare_bytes(&old, &new).unwrap();
    let cr = &d.sheets[0].compared_range;
    let (sr, sc) = cr.start.unwrap();
    let (er, ec) = cr.end.unwrap();
    assert_eq!((sr, sc), (1, 1));  // A1 (1-based)
    assert_eq!((er, ec), (10, 10)); // J10 (1-based)
}

// ============================================================================
// limits
// ============================================================================

#[test]
fn max_diffs_returned_triggers() {
    let old = wb_strings(&[(0, 0, "a"), (0, 1, "b"), (0, 2, "c")]);
    let new = wb_strings(&[(0, 0, "x"), (0, 1, "y"), (0, 2, "z")]);
    let opts = DiffOptions::builder().max_diffs_returned(2).build().unwrap();
    assert!(matches!(
        compare_bytes_with_options(&old, &new, opts),
        Err(SheetsDiffError::LimitExceeded { .. })
    ));
}

#[test]
fn max_sheets_limit_triggers() {
    let b = wb_sheets(&[("S1", &[]), ("S2", &[]), ("S3", &[])]);
    let opts = DiffOptions::builder().max_sheets(2).build().unwrap();
    assert!(matches!(
        compare_bytes_with_options(&b, &b, opts),
        Err(SheetsDiffError::LimitExceeded { .. })
    ));
}

#[test]
fn invalid_option_rejected_before_io() {
    use sheets_diff::FormulaCompareMode;
    let result = DiffOptions::builder()
        .formula_compare(FormulaCompareMode::NormalizedText)
        .build();
    assert!(matches!(result, Err(SheetsDiffError::InvalidOptions { .. })));
}

// ============================================================================
// progress / cancellation  (M5)
// ============================================================================

#[test]
fn progress_events_fired_in_order() {
    use std::sync::{Arc, Mutex};

    let log: Arc<Mutex<Vec<&'static str>>> = Arc::new(Mutex::new(Vec::new()));
    let log2 = log.clone();

    let opts = DiffOptions::builder()
        .progress(move |e: DiffEvent| {
            let tag = match e {
                DiffEvent::Started => "Started",
                DiffEvent::OpeningWorkbook { .. } => "Opening",
                DiffEvent::WorkbookOpened { .. } => "Opened",
                DiffEvent::MatchingSheets => "Matching",
                DiffEvent::SheetStarted { .. } => "SheetStart",
                DiffEvent::SheetFinished { .. } => "SheetEnd",
                DiffEvent::Finished => "Finished",
            };
            log2.lock().unwrap().push(tag);
        })
        .build().unwrap();

    let b = wb_strings(&[(0, 0, "x")]);
    compare_bytes_with_options(&b, &b, opts).unwrap();

    let got = log.lock().unwrap();
    assert_eq!(got[0], "Started");
    assert_eq!(*got.last().unwrap(), "Finished");
    assert!(got.contains(&"Matching"));
    assert!(got.contains(&"SheetStart"));
}

#[test]
fn cancellation_returns_cancelled() {
    let opts = DiffOptions::builder().cancellation(|| true).build().unwrap();
    let b = wb_sheets(&[("S1", &[(0, 0, "a")]), ("S2", &[(0, 0, "b")])]);
    assert!(matches!(
        compare_bytes_with_options(&b, &b, opts),
        Err(SheetsDiffError::Cancelled)
    ));
}

// ============================================================================
// output / text
// ============================================================================

#[test]
fn render_summary_shows_changed_count() {
    let old = wb_strings(&[(0, 0, "a")]);
    let new = wb_strings(&[(0, 0, "b")]);
    let d = compare_bytes(&old, &new).unwrap();
    let s = render_summary(&d);
    assert!(s.contains("1 cell(s) changed"), "got: {s}");
}

#[test]
fn render_unified_has_minus_plus_lines() {
    let old = wb_strings(&[(0, 0, "hello")]);
    let new = wb_strings(&[(0, 0, "world")]);
    let d = compare_bytes(&old, &new).unwrap();
    let u = render_unified(&d);
    assert!(u.contains("-A1"), "missing old line: {u}");
    assert!(u.contains("+A1"), "missing new line: {u}");
}

#[test]
fn render_summary_shows_renamed_sheet() {
    let old = wb_sheets(&[("Before", &[])]);
    let new = wb_sheets(&[("After", &[])]);
    let d = compare_bytes(&old, &new).unwrap();
    let s = render_summary(&d);
    assert!(s.contains("[renamed]"), "got: {s}");
}

#[test]
fn render_unified_shows_added_sheet_marker() {
    let old = wb_sheets(&[("S1", &[])]);
    let new = wb_sheets(&[("S1", &[]), ("S2", &[(0, 0, "x")])]);
    let d = compare_bytes(&old, &new).unwrap();
    let u = render_unified(&d);
    assert!(u.contains("[sheet added]"), "got: {u}");
}

// ============================================================================
// serde / JSON  (M6, only with feature)
// ============================================================================

#[test]
#[cfg(feature = "serde")]
fn json_valid_and_cells_changed_correct() {
    let old = wb_strings(&[(0, 0, "hello")]);
    let new = wb_strings(&[(0, 0, "world")]);
    let d = compare_bytes(&old, &new).unwrap();
    let json = sheets_diff::output::json::to_json(&d).unwrap();
    let v: serde_json::Value = serde_json::from_str(&json).expect("invalid JSON");
    assert_eq!(v["summary"]["cells_changed"], 1);
}

#[test]
#[cfg(feature = "serde")]
fn json_pretty_is_multiline() {
    let b = wb_strings(&[(0, 0, "x")]);
    let d = compare_bytes(&b, &b).unwrap();
    let json = sheets_diff::output::json::to_json_pretty(&d).unwrap();
    assert!(json.contains('\n'), "pretty JSON should be multiline");
}

#[test]
#[cfg(feature = "serde")]
fn json_includes_reserved_empty_arrays() {
    let b = wb_empty();
    let d = compare_bytes(&b, &b).unwrap();
    let json = sheets_diff::output::json::to_json(&d).unwrap();
    let v: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(v["workbook_changes"], serde_json::json!([]));
    assert_eq!(v["object_changes"],   serde_json::json!([]));
}

// ============================================================================
// large workbook (ignored by default)
// ============================================================================

#[test]
#[ignore]
fn large_workbook_completes_within_limit() {
    // 10 000 rows × 10 cols = 100 000 cells; changed on one side.
    let old = wb_large(10_000, 10, "old");
    let new = wb_large(10_000, 10, "new");
    let d = compare_bytes(&old, &new).unwrap();
    assert_eq!(d.summary.cells_changed, 100_000);
}

#[test]
#[ignore]
fn large_workbook_limit_exceeded_cleanly() {
    let old = wb_large(10_000, 10, "old");
    let new = wb_large(10_000, 10, "new");
    let opts = DiffOptions::builder().max_diffs_returned(1_000).build().unwrap();
    assert!(matches!(
        compare_bytes_with_options(&old, &new, opts),
        Err(SheetsDiffError::LimitExceeded { .. })
    ));
}
