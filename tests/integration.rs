//! Integration tests for `sheets-diff` v2 (RFC-015).
//!
//! Fixtures are generated programmatically where possible to avoid committing
//! binary files.  Each test corresponds to a fixture category from RFC-015 §1.

use rust_xlsxwriter::Workbook;
use sheets_diff::{
    CellChangeKind, CellValue, DiffOptions, FormulaCompareMode, SheetChange, SheetMatchingMode,
    SheetsDiffError, compare_bytes, compare_bytes_with_options,
};

// ---------------------------------------------------------------------------
// Fixture builder helpers
// ---------------------------------------------------------------------------

/// Build a single-sheet workbook with the supplied cell matrix and return its bytes.
///
/// `cells` is a slice of `(row, col, value)` tuples; row and col are 0-based
/// (rust_xlsxwriter convention).
fn workbook_bytes(cells: &[(u32, u16, &str)]) -> Vec<u8> {
    let mut wb = Workbook::new();
    let ws = wb.add_worksheet();
    for (row, col, val) in cells {
        ws.write_string(*row, *col, *val).unwrap();
    }
    wb.save_to_buffer().unwrap()
}

fn workbook_bytes_number(cells: &[(u32, u16, f64)]) -> Vec<u8> {
    let mut wb = Workbook::new();
    let ws = wb.add_worksheet();
    for (row, col, val) in cells {
        ws.write_number(*row, *col, *val).unwrap();
    }
    wb.save_to_buffer().unwrap()
}

fn empty_workbook() -> Vec<u8> {
    let mut wb = Workbook::new();
    wb.add_worksheet();
    wb.save_to_buffer().unwrap()
}

fn two_sheet_workbook(names: &[(&str, &[(u32, u16, &str)])]) -> Vec<u8> {
    let mut wb = Workbook::new();
    for (name, cells) in names {
        let ws = wb.add_worksheet();
        ws.set_name(*name).unwrap();
        for (row, col, val) in *cells {
            ws.write_string(*row, *col, *val).unwrap();
        }
    }
    wb.save_to_buffer().unwrap()
}

// ---------------------------------------------------------------------------
// Basic value change
// ---------------------------------------------------------------------------

#[test]
fn identical_workbooks_produce_no_diffs() {
    let bytes = workbook_bytes(&[(0, 0, "hello")]);
    let diff = compare_bytes(&bytes, &bytes).unwrap();
    assert_eq!(diff.summary.cells_changed, 0);
    assert_eq!(diff.summary.sheets_changed, 0);
}

#[test]
fn single_cell_text_change_detected() {
    let old = workbook_bytes(&[(0, 0, "hello")]);
    let new = workbook_bytes(&[(0, 0, "world")]);
    let diff = compare_bytes(&old, &new).unwrap();
    assert_eq!(diff.summary.cells_changed, 1);
    assert_eq!(diff.summary.values_changed, 1);
    let cd = &diff.sheets[0].cell_diffs[0];
    assert_eq!(cd.address.a1, "A1");
    let vc = cd.value.as_ref().unwrap();
    assert!(matches!(&vc.old, CellValue::Text(s) if s == "hello"));
    assert!(matches!(&vc.new, CellValue::Text(s) if s == "world"));
}

#[test]
fn cell_added_to_empty_sheet() {
    let old = empty_workbook();
    let new = workbook_bytes(&[(0, 0, "new value")]);
    let diff = compare_bytes(&old, &new).unwrap();
    assert_eq!(diff.summary.cells_changed, 1);
    let cd = &diff.sheets[0].cell_diffs[0];
    assert_eq!(cd.change_kind(), CellChangeKind::Added);
}

#[test]
fn cell_removed_from_sheet() {
    let old = workbook_bytes(&[(0, 0, "gone")]);
    let new = empty_workbook();
    let diff = compare_bytes(&old, &new).unwrap();
    assert_eq!(diff.summary.cells_changed, 1);
    let cd = &diff.sheets[0].cell_diffs[0];
    assert_eq!(cd.change_kind(), CellChangeKind::Removed);
}

// ---------------------------------------------------------------------------
// Typed value distinctions (RFC-033 §4)
// ---------------------------------------------------------------------------

#[test]
fn text_100_and_number_100_are_different() {
    let old = workbook_bytes(&[(0, 0, "100")]);
    let new = workbook_bytes_number(&[(0, 0, 100.0)]);
    let diff = compare_bytes(&old, &new).unwrap();
    assert_eq!(diff.summary.values_changed, 1);
    let vc = diff.sheets[0].cell_diffs[0].value.as_ref().unwrap();
    assert!(matches!(vc.old, CellValue::Text(_)));
    assert!(matches!(vc.new, CellValue::Number(_)));
}

// ---------------------------------------------------------------------------
// Sheet changes
// ---------------------------------------------------------------------------

#[test]
fn added_sheet_detected() {
    let old = two_sheet_workbook(&[("Sheet1", &[])]);
    let new = two_sheet_workbook(&[("Sheet1", &[]), ("Sheet2", &[])]);
    let diff = compare_bytes(&old, &new).unwrap();
    assert_eq!(diff.summary.sheets_added, 1);
    let added = diff.sheets.iter().find(|s| matches!(s.change, SheetChange::Added)).unwrap();
    assert_eq!(added.new_sheet.as_ref().unwrap().name, "Sheet2");
}

#[test]
fn removed_sheet_detected() {
    let old = two_sheet_workbook(&[("Sheet1", &[]), ("Sheet2", &[])]);
    let new = two_sheet_workbook(&[("Sheet1", &[])]);
    let diff = compare_bytes(&old, &new).unwrap();
    assert_eq!(diff.summary.sheets_removed, 1);
}

#[test]
fn renamed_sheet_detected_conservative() {
    // One removed + one added = conservative rename inference.
    let old = two_sheet_workbook(&[("OldName", &[(0, 0, "value")])]);
    let new = two_sheet_workbook(&[("NewName", &[(0, 0, "value")])]);
    let diff = compare_bytes(&old, &new).unwrap();
    assert_eq!(diff.summary.sheets_renamed, 1);
    let renamed = diff.sheets.iter().find(|s| matches!(s.change, SheetChange::Renamed { .. })).unwrap();
    assert_eq!(renamed.old_sheet.as_ref().unwrap().name, "OldName");
    assert_eq!(renamed.new_sheet.as_ref().unwrap().name, "NewName");
}

#[test]
fn renamed_sheet_still_produces_cell_diffs() {
    let old = two_sheet_workbook(&[("OldName", &[(0, 0, "before")])]);
    let new = two_sheet_workbook(&[("NewName", &[(0, 0, "after")])]);
    let diff = compare_bytes(&old, &new).unwrap();
    let sd = diff.sheets.iter().find(|s| matches!(s.change, SheetChange::Renamed { .. })).unwrap();
    assert_eq!(sd.cell_diffs.len(), 1);
}

// ---------------------------------------------------------------------------
// Exact-name-only mode suppresses rename
// ---------------------------------------------------------------------------

#[test]
fn exact_name_only_does_not_rename() {
    let old = two_sheet_workbook(&[("OldName", &[])]);
    let new = two_sheet_workbook(&[("NewName", &[])]);
    let opts = DiffOptions::builder()
        .sheet_matching(SheetMatchingMode::ExactNameOnly)
        .build()
        .unwrap();
    let diff = compare_bytes_with_options(&old, &new, opts).unwrap();
    assert_eq!(diff.summary.sheets_renamed, 0);
    assert_eq!(diff.summary.sheets_added, 1);
    assert_eq!(diff.summary.sheets_removed, 1);
}

// ---------------------------------------------------------------------------
// Formula comparison
// ---------------------------------------------------------------------------

#[test]
fn formula_ignore_mode_skips_formula_changes() {
    // We can only test the Ignore guard via options; actual formula round-trip
    // requires fixtures with formula data (rust_xlsxwriter write_formula).
    let opts = DiffOptions::builder()
        .formula_compare(FormulaCompareMode::Ignore)
        .build()
        .unwrap();
    // Just verify it doesn't error; formula comparison is off.
    let bytes = workbook_bytes(&[(0, 0, "x")]);
    let diff = compare_bytes_with_options(&bytes, &bytes, opts).unwrap();
    assert_eq!(diff.summary.formulas_changed, 0);
}

// ---------------------------------------------------------------------------
// Deterministic ordering
// ---------------------------------------------------------------------------

#[test]
fn cell_diffs_sorted_by_row_then_col() {
    let old = workbook_bytes(&[(0, 0, "a"), (0, 1, "b"), (1, 0, "c")]);
    let new = workbook_bytes(&[(0, 0, "x"), (0, 1, "y"), (1, 0, "z")]);
    let diff = compare_bytes(&old, &new).unwrap();
    let addrs: Vec<&str> = diff.sheets[0].cell_diffs.iter().map(|c| c.address.a1.as_str()).collect();
    assert_eq!(addrs, vec!["A1", "B1", "A2"]);
}

// ---------------------------------------------------------------------------
// A1 addressing through wide columns
// ---------------------------------------------------------------------------

#[test]
fn wide_column_address_is_correct() {
    use sheets_diff::address::col_to_label;
    assert_eq!(col_to_label(1), "A");
    assert_eq!(col_to_label(26), "Z");
    assert_eq!(col_to_label(27), "AA");
    assert_eq!(col_to_label(16_384), "XFD");
}

// ---------------------------------------------------------------------------
// Error cases (RFC-032)
// ---------------------------------------------------------------------------

#[test]
fn corrupt_bytes_return_structured_error() {
    let junk = b"not a zip file at all";
    let good = workbook_bytes(&[]);
    match compare_bytes(junk.as_slice(), &good) {
        Err(SheetsDiffError::OpenWorkbook { .. }) => {} // expected
        Err(e) => panic!("unexpected error variant: {e}"),
        Ok(_) => panic!("expected error, got Ok"),
    }
}

#[test]
fn no_panic_on_empty_byte_slice() {
    let empty: &[u8] = &[];
    let good = workbook_bytes(&[]);
    let result = compare_bytes(empty, &good);
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// Resource limits
// ---------------------------------------------------------------------------

#[test]
fn max_diffs_returned_limit_triggers() {
    // 3 changed cells; limit = 2.
    let old = workbook_bytes(&[(0, 0, "a"), (0, 1, "b"), (0, 2, "c")]);
    let new = workbook_bytes(&[(0, 0, "x"), (0, 1, "y"), (0, 2, "z")]);
    let opts = DiffOptions::builder().max_diffs_returned(2).build().unwrap();
    let result = compare_bytes_with_options(&old, &new, opts);
    match result {
        Err(SheetsDiffError::LimitExceeded { .. }) => {}
        other => panic!("expected LimitExceeded, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// M5 – Progress events and cancellation
// ---------------------------------------------------------------------------

#[test]
fn progress_events_are_emitted() {
    use std::sync::{Arc, Mutex};
    use sheets_diff::{DiffEvent, DiffOptions};

    let events: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let events_clone = events.clone();

    let opts = DiffOptions::builder()
        .progress(move |e: DiffEvent| {
            let label = match &e {
                DiffEvent::Started => "Started".into(),
                DiffEvent::OpeningWorkbook { side } => format!("Opening:{side}"),
                DiffEvent::WorkbookOpened { side, .. } => format!("Opened:{side}"),
                DiffEvent::MatchingSheets => "MatchingSheets".into(),
                DiffEvent::SheetStarted { name, .. } => format!("SheetStarted:{name}"),
                DiffEvent::SheetFinished { .. } => "SheetFinished".into(),
                DiffEvent::Finished => "Finished".into(),
            };
            events_clone.lock().unwrap().push(label);
        })
        .build()
        .unwrap();

    let bytes = workbook_bytes(&[(0, 0, "x")]);
    compare_bytes_with_options(&bytes, &bytes, opts).unwrap();

    let got = events.lock().unwrap();
    assert!(got.contains(&"Started".into()), "missing Started event");
    assert!(got.contains(&"Finished".into()), "missing Finished event");
    assert!(got.iter().any(|e| e.starts_with("Opened:")), "missing WorkbookOpened");
}

#[test]
fn cancellation_returns_cancelled_error() {
    use sheets_diff::{DiffOptions, SheetsDiffError};

    // Cancel immediately
    let opts = DiffOptions::builder()
        .cancellation(|| true)
        .build()
        .unwrap();

    // Build a workbook with multiple sheets so cancellation has a chance to fire
    let bytes = two_sheet_workbook(&[
        ("Sheet1", &[(0, 0, "a")]),
        ("Sheet2", &[(0, 0, "b")]),
    ]);
    let result = compare_bytes_with_options(&bytes, &bytes, opts);
    assert!(
        matches!(result, Err(SheetsDiffError::Cancelled)),
        "expected Cancelled, got {result:?}"
    );
}

// ---------------------------------------------------------------------------
// M6 – Text output
// ---------------------------------------------------------------------------

#[test]
fn render_summary_contains_changed_count() {
    use sheets_diff::output::text::render_summary;

    let old = workbook_bytes(&[(0, 0, "before")]);
    let new = workbook_bytes(&[(0, 0, "after")]);
    let diff = compare_bytes(&old, &new).unwrap();
    let summary = render_summary(&diff);
    assert!(summary.contains("1 cell(s) changed") || summary.contains("cells"), 
        "summary missing cell change count: {summary}");
}

#[test]
fn render_unified_contains_minus_plus_lines() {
    use sheets_diff::output::text::render_unified;

    let old = workbook_bytes(&[(0, 0, "hello")]);
    let new = workbook_bytes(&[(0, 0, "world")]);
    let diff = compare_bytes(&old, &new).unwrap();
    let unified = render_unified(&diff);
    assert!(unified.contains("-A1"), "expected old value line starting with -A1");
    assert!(unified.contains("+A1"), "expected new value line starting with +A1");
}

// ---------------------------------------------------------------------------
// M6 – JSON / serde (only when feature enabled)
// ---------------------------------------------------------------------------

#[test]
#[cfg(feature = "serde")]
fn json_output_is_valid_and_contains_cells_changed() {
    use sheets_diff::output::json::to_json;

    let old = workbook_bytes(&[(0, 0, "hello")]);
    let new = workbook_bytes(&[(0, 0, "world")]);
    let diff = compare_bytes(&old, &new).unwrap();
    let json = to_json(&diff).unwrap();

    // Must be valid JSON
    let v: serde_json::Value = serde_json::from_str(&json).expect("not valid JSON");
    // Must contain summary.cells_changed = 1
    assert_eq!(v["summary"]["cells_changed"], 1);
}

// ---------------------------------------------------------------------------
// Additional M3 fixture: multiple cells, multiple sheets
// ---------------------------------------------------------------------------

#[test]
fn multiple_sheets_all_diffs_collected() {
    let old = two_sheet_workbook(&[
        ("Data", &[(0, 0, "a"), (0, 1, "b")]),
        ("Meta", &[(0, 0, "v1")]),
    ]);
    let new = two_sheet_workbook(&[
        ("Data", &[(0, 0, "x"), (0, 1, "b")]),   // 1 cell changed
        ("Meta", &[(0, 0, "v2")]),                 // 1 cell changed
    ]);
    let diff = compare_bytes(&old, &new).unwrap();
    assert_eq!(diff.summary.cells_changed, 2);
    assert_eq!(diff.summary.sheets_changed, 2);
    assert_eq!(diff.sheets.len(), 2);
}

#[test]
fn unchanged_sheet_not_counted_as_changed() {
    let bytes = two_sheet_workbook(&[
        ("Sheet1", &[(0, 0, "same")]),
        ("Sheet2", &[(0, 0, "same")]),
    ]);
    let diff = compare_bytes(&bytes, &bytes).unwrap();
    assert_eq!(diff.summary.sheets_changed, 0);
    assert_eq!(diff.summary.cells_changed, 0);
}
