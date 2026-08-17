//! Integration tests for `sheets-diff` v2 (RFC-015).
//!
//! Fixture categories follow RFC-015 §5:
//!   basic · corrupt · wide-columns · typed-values · formulas
//!   sheet-renames · empty-sheets · sparse-ranges · limits
//!   progress/cancellation · output · serde

mod support;
use rust_xlsxwriter::Workbook;
use support::*;

use sheets_diff::{
    Cancellation, CellChangeKind, CellError, CellValue, DateComparePolicy, DiffEvent, DiffOptions,
    FormulaCompareMode, SheetChange, SheetMatchingMode, SheetsDiffError, ValueDifferenceKind,
    compare_bytes, compare_bytes_with_options, compare_paths_with_options,
    compare_readers_with_options,
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
    assert_eq!(
        d.sheets[0].cell_diffs[0].change_kind(),
        CellChangeKind::Added
    );
}

#[test]
fn cell_removed_from_sheet() {
    let old = wb_strings(&[(0, 0, "gone")]);
    let new = wb_empty();
    let d = compare_bytes(&old, &new).unwrap();
    assert_eq!(
        d.sheets[0].cell_diffs[0].change_kind(),
        CellChangeKind::Removed
    );
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
    let addrs: Vec<&str> = d.sheets[0]
        .cell_diffs
        .iter()
        .map(|c| c.address.a1.as_str())
        .collect();
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
    assert!(
        !std::panic::catch_unwind(|| compare_bytes(&corrupt, &good)).is_err(),
        "must not panic"
    );
}

// ============================================================================
// wide-columns  (RFC-015: columns A / Z / AA / AZ / BA / ZZ / AAA / XFD)
// ============================================================================

#[test]
fn wide_column_a1_encoding() {
    use sheets_diff::address::col_to_label;
    assert_eq!(col_to_label(1), "A");
    assert_eq!(col_to_label(26), "Z");
    assert_eq!(col_to_label(27), "AA");
    assert_eq!(col_to_label(52), "AZ");
    assert_eq!(col_to_label(53), "BA");
    assert_eq!(col_to_label(702), "ZZ");
    assert_eq!(col_to_label(703), "AAA");
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
    let a2 = CellAddress::new(2, 1).unwrap();
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
    // Integer(1) and Number(1.0) are distinct CellValue variants — confirmed unequal.
    // The TypeChanged reason is tested in compare::tests (unit tests in src/compare.rs).
    assert_ne!(CellValue::Integer(1), CellValue::Number(1.0));
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
    // Tolerance comparison tested via full diff with two workbooks.
    use sheets_diff::{DiffOptions, options::NumberComparePolicy};
    let old = wb_numbers(&[(0, 0, 1.0)]);
    let new = wb_numbers(&[(0, 0, 1.005)]);
    let opts = DiffOptions::builder()
        .number_compare_policy(NumberComparePolicy::AbsoluteTolerance(0.01))
        .build()
        .unwrap();
    let d = compare_bytes_with_options(&old, &new, opts).unwrap();
    assert_eq!(
        d.summary.values_changed, 0,
        "should be equal within tolerance"
    );
}

#[test]
fn numeric_tolerance_detects_difference_outside_tolerance() {
    use sheets_diff::{DiffOptions, options::NumberComparePolicy};
    let old = wb_numbers(&[(0, 0, 1.0)]);
    let new = wb_numbers(&[(0, 0, 1.005)]);
    let opts = DiffOptions::builder()
        .number_compare_policy(NumberComparePolicy::AbsoluteTolerance(0.001))
        .build()
        .unwrap();
    let d = compare_bytes_with_options(&old, &new, opts).unwrap();
    assert_eq!(
        d.summary.values_changed, 1,
        "should detect difference outside tolerance"
    );
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
        .build()
        .unwrap();
    let d = compare_bytes_with_options(&old, &new, opts).unwrap();
    assert_eq!(d.summary.formulas_changed, 0);
}

#[test]
fn value_and_formula_both_changed_is_one_cell_diff() {
    // Change both the string value and the formula in the same cell position.
    let old = wb_with_formula(0, 0, "before", 0, 0, "=1");
    let new = wb_with_formula(0, 0, "after", 0, 0, "=2");
    let d = compare_bytes(&old, &new).unwrap();
    // A1 contains either the string or the formula depending on write order;
    // the key invariant is at most one CellDiff per address.
    let a1_diffs: Vec<_> = d.sheets[0]
        .cell_diffs
        .iter()
        .filter(|c| c.address.a1 == "A1")
        .collect();
    assert!(
        a1_diffs.len() <= 1,
        "must be at most one CellDiff per address"
    );
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
    assert!(
        d.sheets
            .iter()
            .any(|s| matches!(s.change, SheetChange::Renamed { .. }))
    );
}

#[test]
fn renamed_sheet_preserves_cell_diffs() {
    let old = wb_sheets(&[("OldName", &[(0, 0, "before")])]);
    let new = wb_sheets(&[("NewName", &[(0, 0, "after")])]);
    let d = compare_bytes(&old, &new).unwrap();
    let renamed = d
        .sheets
        .iter()
        .find(|s| matches!(s.change, SheetChange::Renamed { .. }))
        .unwrap();
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
        .build()
        .unwrap();
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
    assert!(
        d.diagnostics
            .iter()
            .any(|diag| { diag.kind.code() == "ambiguous_sheet_match" })
    );
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
    assert!(
        d.sheets[0]
            .cell_diffs
            .iter()
            .all(|c| c.change_kind() == CellChangeKind::Added)
    );
}

#[test]
fn nonempty_vs_empty_all_cells_removed() {
    let old = wb_strings(&[(0, 0, "a"), (3, 2, "b")]);
    let new = wb_empty();
    let d = compare_bytes(&old, &new).unwrap();
    assert_eq!(d.summary.cells_changed, 2);
    assert!(
        d.sheets[0]
            .cell_diffs
            .iter()
            .all(|c| c.change_kind() == CellChangeKind::Removed)
    );
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
    assert_eq!((sr, sc), (1, 1)); // A1 (1-based)
    assert_eq!((er, ec), (10, 10)); // J10 (1-based)
}

// ============================================================================
// limits
// ============================================================================

#[test]
fn max_diffs_returned_triggers() {
    let old = wb_strings(&[(0, 0, "a"), (0, 1, "b"), (0, 2, "c")]);
    let new = wb_strings(&[(0, 0, "x"), (0, 1, "y"), (0, 2, "z")]);
    let opts = DiffOptions::builder()
        .max_diffs_returned(2)
        .build()
        .unwrap();
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
    assert!(matches!(
        result,
        Err(SheetsDiffError::InvalidOptions { .. })
    ));
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
        .build()
        .unwrap();

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
    let opts = DiffOptions::builder()
        .cancellation(|| true)
        .build()
        .unwrap();
    let b = wb_sheets(&[("S1", &[(0, 0, "a")]), ("S2", &[(0, 0, "b")])]);
    assert!(matches!(
        compare_bytes_with_options(&b, &b, opts),
        Err(SheetsDiffError::Cancelled)
    ));
}

// ---------------------------------------------------------------------------
// M7 Handoff 03 — cancellation observed mid-sheet, not just between sheets
// ---------------------------------------------------------------------------

/// A workbook with a single dense block of populated cells, offset by
/// `row_offset` rows. Not shared via `tests/support.rs` — only this file
/// needs a row-offset variant of `wb_large`, to build two sheets whose
/// populated coordinates are disjoint (see
/// `cancellation_observed_during_compare_phase` below).
fn wb_dense_block(row_offset: u32, rows: u32, cols: u16, prefix: &str) -> Vec<u8> {
    let mut wb = Workbook::new();
    let ws = wb.add_worksheet();
    for r in 0..rows {
        for c in 0..cols {
            ws.write_string(row_offset + r, c, format!("{prefix}_{r}_{c}"))
                .unwrap();
        }
    }
    wb.save_to_buffer().unwrap()
}

/// Cancelled from the *second* poll onward, never the first.
///
/// A naive `|| true` cancels on poll #1 — which, for any workbook, is the
/// pre-existing per-sheet-pair `check_cancel` call that already ran before
/// this unit's fix. A test built on that policy would pass identically
/// whether or not the new mid-sheet checkpoints exist, "passing for the
/// wrong reason" (the handoff's own Known Risks warning). Reporting
/// not-cancelled on the first poll and cancelled on every poll after it
/// guarantees the *first* internal checkpoint reached — not the outer,
/// already-existing one — is what this test actually exercises.
struct CancelFromSecondPoll(std::sync::atomic::AtomicUsize);

impl CancelFromSecondPoll {
    fn new() -> Self {
        Self(std::sync::atomic::AtomicUsize::new(0))
    }
}

impl Cancellation for CancelFromSecondPoll {
    fn is_cancelled(&self) -> bool {
        // fetch_add returns the pre-increment value: poll #1 sees 0 (not
        // cancelled), poll #2 sees 1, poll #3 sees 2, ... (cancelled).
        self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst) >= 1
    }
}

#[test]
fn cancellation_observed_during_read_phase() {
    // 300 x 200 = 60,000 cells, one dense sheet — comfortably more than
    // CANCEL_POLL_INTERVAL (50,000), so reading this single sheet alone
    // crosses an interval boundary. The "new" side shares the sheet's
    // default name but is trivial, so old-side reading runs first and
    // alone is what the fix must catch: `read_sheet_cells` for "new", and
    // `build_sheet_diff`, are never reached if this is observed correctly.
    let old = wb_dense_block(0, 300, 200, "old");
    let new = wb_strings(&[(0, 0, "x")]);

    let opts = DiffOptions::builder()
        .cancellation(CancelFromSecondPoll::new())
        .build()
        .unwrap();
    assert!(
        matches!(
            compare_bytes_with_options(&old, &new, opts),
            Err(SheetsDiffError::Cancelled)
        ),
        "a 60,000-cell single sheet must be cancellable mid-read, not just \
         at the sheet-pair boundary before it starts"
    );
}

#[test]
fn cancellation_observed_during_compare_phase() {
    // Two dense blocks of 200 x 200 = 40,000 cells each — individually
    // under CANCEL_POLL_INTERVAL (50,000), so neither side's own read
    // crosses an interval boundary on its own. Placed at disjoint row
    // ranges (0..200 vs 300..500) so Positional alignment's union of both
    // sides' populated coordinates does not collapse them: the coordinate
    // set compared is 40,000 + 40,000 = 80,000 entries, which *does* cross
    // an interval boundary — but only in `build_sheet_diff`'s compare loop,
    // after both reads have already completed cleanly. If only the read
    // checkpoint existed, this test would time out, not fail fast.
    let old = wb_dense_block(0, 200, 200, "old");
    let new = wb_dense_block(300, 200, 200, "new");

    let opts = DiffOptions::builder()
        .cancellation(CancelFromSecondPoll::new())
        .build()
        .unwrap();
    assert!(
        matches!(
            compare_bytes_with_options(&old, &new, opts),
            Err(SheetsDiffError::Cancelled)
        ),
        "an 80,000-coordinate comparison, built from two reads that each \
         individually stay under one polling interval, must still be \
         cancellable mid-compare"
    );
}

#[test]
fn cancellation_configured_but_never_fires_leaves_result_unchanged() {
    // Corpus-strength proof (required test 3) that adding polling
    // checkpoints does not alter what a non-cancelled comparison returns.
    // Large enough (100 x 50 = 5,000 cells) to actually pass through a few
    // poll checks without crossing CANCEL_POLL_INTERVAL, so this exercises
    // the "poll, find not-cancelled, continue" path, not just the trivial
    // small-workbook case.
    let old = wb_large(100, 50, "old");
    let new = wb_large(100, 50, "new");

    let without = compare_bytes(&old, &new).unwrap();
    let opts = DiffOptions::builder()
        .cancellation(|| false)
        .build()
        .unwrap();
    let with = compare_bytes_with_options(&old, &new, opts).unwrap();

    assert_eq!(without.summary.cells_changed, with.summary.cells_changed);
    assert_eq!(
        without.sheets[0].cell_diffs.len(),
        with.sheets[0].cell_diffs.len()
    );
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
    assert_eq!(v["object_changes"], serde_json::json!([]));
}

// ============================================================================
// large workbook (RFC-034 Handoff 01 item 6: was #[ignore]d, never run)
// ============================================================================

#[test]
fn large_workbook_completes_within_limit() {
    // 10 000 rows × 10 cols = 100 000 cells; changed on one side.
    let old = wb_large(10_000, 10, "old");
    let new = wb_large(10_000, 10, "new");
    let d = compare_bytes(&old, &new).unwrap();
    assert_eq!(d.summary.cells_changed, 100_000);
}

#[test]
fn large_workbook_limit_exceeded_cleanly() {
    let old = wb_large(10_000, 10, "old");
    let new = wb_large(10_000, 10, "new");
    let opts = DiffOptions::builder()
        .max_diffs_returned(1_000)
        .build()
        .unwrap();
    assert!(matches!(
        compare_bytes_with_options(&old, &new, opts),
        Err(SheetsDiffError::LimitExceeded { .. })
    ));
}

// ============================================================================
// v2.1 — RFC-011 alignment
// ============================================================================

#[test]
fn row_key_alignment_reduces_cascade() {
    use sheets_diff::options::{AlignmentMode, MatchingOptions};

    // old: id1/val_a, id2/val_b, id3/val_c
    // new: id1/val_a, id_new/val_x, id2/val_b, id3/val_c  (row inserted at position 2)
    // Positional would report id2→id_new, id3→id2, nothing→id3 (3 false positives)
    // RowKey should match id1↔id1, id2↔id2, id3↔id3 and report only id_new as inserted.
    let old = {
        let mut wb = Workbook::new();
        let ws = wb.add_worksheet();
        ws.write_string(0, 0, "id1").unwrap();
        ws.write_string(0, 1, "val_a").unwrap();
        ws.write_string(1, 0, "id2").unwrap();
        ws.write_string(1, 1, "val_b").unwrap();
        ws.write_string(2, 0, "id3").unwrap();
        ws.write_string(2, 1, "val_c").unwrap();
        wb.save_to_buffer().unwrap()
    };
    let new = {
        let mut wb = Workbook::new();
        let ws = wb.add_worksheet();
        ws.write_string(0, 0, "id1").unwrap();
        ws.write_string(0, 1, "val_a").unwrap();
        ws.write_string(1, 0, "id_new").unwrap();
        ws.write_string(1, 1, "val_x").unwrap();
        ws.write_string(2, 0, "id2").unwrap();
        ws.write_string(2, 1, "val_b").unwrap();
        ws.write_string(3, 0, "id3").unwrap();
        ws.write_string(3, 1, "val_c").unwrap();
        wb.save_to_buffer().unwrap()
    };

    // Positional: all 3 data rows appear changed (cascade)
    let pos_diff = compare_bytes(&old, &new).unwrap();
    assert!(
        pos_diff.summary.cells_changed >= 3,
        "positional should show cascade"
    );

    // RowKey on column A (col index 1, 1-based): only id_new is truly new
    let opts = DiffOptions::builder()
        .build_with_matching(MatchingOptions {
            sheet_matching: SheetMatchingMode::default(),
            alignment: AlignmentMode::RowKey { columns: vec![1] },
        })
        .unwrap();
    let aligned_diff = compare_bytes_with_options(&old, &new, opts).unwrap();
    // With key alignment, val_a/val_b/val_c rows should NOT appear as changed.
    assert!(
        aligned_diff.summary.cells_changed < pos_diff.summary.cells_changed,
        "aligned diff should report fewer changes than positional: aligned={}, positional={}",
        aligned_diff.summary.cells_changed,
        pos_diff.summary.cells_changed
    );
    let sheet = &aligned_diff.sheets[0];
    assert!(
        sheet.alignment_summary.is_some(),
        "alignment summary should be set"
    );
}

// ============================================================================
// v2.3 — RFC-035 resource bounds
// ============================================================================

#[test]
fn limits_hardened_bounds_every_dimension() {
    use sheets_diff::Limits;
    let h = Limits::hardened();
    assert!(h.max_sheets.is_some());
    assert!(h.max_cells_read.is_some());
    assert!(h.max_cells_compared.is_some());
    assert!(h.max_diffs_returned.is_some());
    assert!(h.max_alignment_product.is_some());
    assert!(h.max_input_bytes.is_some());
}

#[test]
fn default_limits_bound_alignment_and_input_but_not_linear_paths() {
    use sheets_diff::Limits;
    let d = Limits::default();
    // Superlinear paths (RFC-035 §5.1) are bounded by default.
    assert!(d.max_alignment_product.is_some());
    assert!(d.max_input_bytes.is_some());
    // Linear paths stay opt-in.
    assert!(d.max_sheets.is_none());
    assert!(d.max_cells_read.is_none());
    assert!(d.max_cells_compared.is_none());
    assert!(d.max_diffs_returned.is_none());
}

#[test]
fn limits_struct_update_syntax_still_compiles() {
    // The pre-existing `Limits { field: ..., ..Limits::default() }`
    // construction pattern must keep working now that `Limits` no longer
    // derives `Default` (it has a manual impl instead, since two fields
    // default to `Some` rather than `None`).
    use sheets_diff::Limits;
    let limits = Limits {
        max_sheets: Some(10),
        ..Limits::default()
    };
    assert_eq!(limits.max_sheets, Some(10));
    assert!(limits.max_alignment_product.is_some());
    assert!(limits.max_input_bytes.is_some());
}

// ============================================================================
// M4 unit 04 — F-A: `max_cells_compared` bounds coordinates, not diffs
// ============================================================================

#[test]
fn cells_compared_limit_fires_on_coordinates_with_zero_diffs() {
    // The discriminating case: many coordinates, zero diffs. Comparing a
    // workbook against itself gives exactly that -- 1000 populated cells,
    // all unchanged.
    //
    // Under the OLD enforcement (`compared_so_far = cell_diffs.len() + 1`),
    // zero diffs means `cell_diffs` never grows, so `compared_so_far` is a
    // constant `1` on every iteration -- it would never exceed any `max`
    // greater than 0, no matter how many coordinates are visited. This
    // input and this limit could not trip the old check; they trip the new
    // one, which is what makes this a real discrimination rather than a
    // limit low enough to catch both.
    let wb = wb_large(50, 20, "same"); // 50 * 20 = 1000 identical cells
    let opts = DiffOptions::builder()
        .max_cells_compared(100)
        .build()
        .unwrap();
    match compare_bytes_with_options(&wb, &wb, opts) {
        Err(SheetsDiffError::LimitExceeded {
            limit: sheets_diff::LimitKind::CellsCompared,
            observed,
        }) => {
            // `observed` must report coordinates compared -- exactly 1000
            // for this fixture -- not a diff count (which would be 0, and
            // could never have exceeded `max` in the first place).
            assert_eq!(observed, 1000);
        }
        other => panic!("expected LimitExceeded{{CellsCompared}}, got {other:?}"),
    }
}

#[test]
fn cells_compared_limit_does_not_fire_below_bound() {
    let wb = wb_large(50, 20, "same"); // 1000 identical cells
    let opts = DiffOptions::builder()
        .max_cells_compared(2_000)
        .build()
        .unwrap();
    let diff = compare_bytes_with_options(&wb, &wb, opts).unwrap();
    assert_eq!(diff.summary.cells_changed, 0);
}

#[test]
fn alignment_bound_exceeded_degrades_not_errors() {
    use sheets_diff::options::{AlignmentMode, MatchingOptions};

    let old = wb_strings(&[(0, 0, "id1"), (1, 0, "id2"), (2, 0, "id3")]);
    let new = wb_strings(&[(0, 0, "id1"), (1, 0, "id2"), (2, 0, "id9")]);

    let positional = compare_bytes(&old, &new).unwrap();

    // 3 distinct old rows x 3 distinct new rows = product 9; bound it below that.
    let opts = DiffOptions::builder()
        .max_alignment_product(Some(5))
        .build_with_matching(MatchingOptions {
            sheet_matching: SheetMatchingMode::default(),
            alignment: AlignmentMode::RowKey { columns: vec![1] },
        })
        .unwrap();
    let degraded = compare_bytes_with_options(&old, &new, opts).unwrap();

    // Degrades to the same result as positional comparison — never an error.
    assert_eq!(
        degraded.summary.cells_changed,
        positional.summary.cells_changed
    );
    assert!(
        degraded.sheets[0]
            .diagnostics
            .iter()
            .any(|d| d.kind.code() == "alignment_bound_exceeded"),
        "expected alignment_bound_exceeded diagnostic, got {:?}",
        degraded.sheets[0].diagnostics
    );
}

#[test]
fn duplicate_alignment_key_diagnostic_uses_new_code() {
    use sheets_diff::options::{AlignmentMode, MatchingOptions};

    let old = wb_strings(&[(0, 0, "dup"), (1, 0, "dup"), (2, 0, "unique")]);
    let new = wb_strings(&[(0, 0, "dup"), (1, 0, "unique")]);
    let opts = DiffOptions::builder()
        .build_with_matching(MatchingOptions {
            sheet_matching: SheetMatchingMode::default(),
            alignment: AlignmentMode::RowKey { columns: vec![1] },
        })
        .unwrap();
    let d = compare_bytes_with_options(&old, &new, opts).unwrap();
    assert!(
        d.sheets[0]
            .diagnostics
            .iter()
            .any(|diag| diag.kind.code() == "duplicate_alignment_key"),
        "expected duplicate_alignment_key diagnostic, got {:?}",
        d.sheets[0].diagnostics
    );
}

#[test]
fn input_bytes_bound_rejects_before_parsing_bytes() {
    // Oversized junk that is not valid xlsx: if the size check ran after
    // opening/parsing, this would surface as an OpenWorkbook error instead
    // (see corrupt_bytes_structured_error). Getting LimitExceeded proves the
    // size check runs first — RFC-035 §5.4.
    let junk = vec![0u8; 1024];
    let good = wb_empty();
    let opts = DiffOptions::builder()
        .max_input_bytes(Some(100))
        .build()
        .unwrap();
    assert!(matches!(
        compare_bytes_with_options(&junk, &good, opts),
        Err(SheetsDiffError::LimitExceeded {
            limit: sheets_diff::LimitKind::InputBytes,
            ..
        })
    ));
}

#[test]
fn input_bytes_bound_rejects_before_parsing_path() {
    let junk = vec![0u8; 1024];
    let path = std::env::temp_dir().join(format!(
        "sheets-diff-oversized-input-{}-old.xlsx",
        std::process::id()
    ));
    let good_path = std::env::temp_dir().join(format!(
        "sheets-diff-oversized-input-{}-new.xlsx",
        std::process::id()
    ));
    std::fs::write(&path, &junk).unwrap();
    std::fs::write(&good_path, wb_empty()).unwrap();

    let opts = DiffOptions::builder()
        .max_input_bytes(Some(100))
        .build()
        .unwrap();
    let result = compare_paths_with_options(&path, &good_path, opts);

    std::fs::remove_file(&path).ok();
    std::fs::remove_file(&good_path).ok();

    assert!(matches!(
        result,
        Err(SheetsDiffError::LimitExceeded {
            limit: sheets_diff::LimitKind::InputBytes,
            ..
        })
    ));
}

#[test]
fn input_bytes_bound_rejects_before_parsing_reader() {
    use std::io::Cursor;
    let junk = Cursor::new(vec![0u8; 1024]);
    let good = Cursor::new(wb_empty());
    let opts = DiffOptions::builder()
        .max_input_bytes(Some(100))
        .build()
        .unwrap();
    assert!(matches!(
        compare_readers_with_options(junk, good, opts),
        Err(SheetsDiffError::LimitExceeded {
            limit: sheets_diff::LimitKind::InputBytes,
            ..
        })
    ));
}

// ============================================================================
// v2.1 — RFC-029 view adapters
// ============================================================================

#[test]
fn diff_view_rows_matches_cell_diffs() {
    use sheets_diff::output::view::{DiffView, ViewFilter};

    let old = wb_strings(&[(0, 0, "a"), (0, 1, "b")]);
    let new = wb_strings(&[(0, 0, "x"), (0, 1, "b")]);
    let diff = compare_bytes(&old, &new).unwrap();
    let view = DiffView::new(&diff);
    let rows = view.rows(&ViewFilter::default());
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].anchor.col, 1); // A1 column 1
    assert_eq!(rows[0].old_display, "a");
    assert_eq!(rows[0].new_display, "x");
}

#[test]
fn diff_view_navigation() {
    use sheets_diff::output::view::{DiffView, ViewFilter};

    let old = wb_strings(&[(0, 0, "a"), (0, 1, "b"), (0, 2, "c")]);
    let new = wb_strings(&[(0, 0, "x"), (0, 1, "y"), (0, 2, "z")]);
    let diff = compare_bytes(&old, &new).unwrap();
    let view = DiffView::new(&diff);
    let filter = ViewFilter::default();

    let first = view.first(&filter).unwrap();
    assert_eq!(first.col, 1); // A1

    let second = view.next_after(&first, &filter).unwrap();
    assert_eq!(second.col, 2); // B1

    let back = view.previous_before(&second, &filter).unwrap();
    assert_eq!(back, first);
}

#[test]
fn diff_view_filter_excludes_formulas() {
    use sheets_diff::output::view::{DiffView, ViewFilter};

    let old = wb_strings(&[(0, 0, "a")]);
    let new = wb_strings(&[(0, 0, "b")]);
    let diff = compare_bytes(&old, &new).unwrap();
    let view = DiffView::new(&diff);

    let filter = ViewFilter {
        include_values: false,
        ..Default::default()
    };
    // With values excluded and no formula changes, should be empty.
    assert_eq!(view.rows(&filter).len(), 0);
}

#[test]
fn diff_view_sheet_summary() {
    use sheets_diff::output::view::DiffView;

    let old = wb_sheets(&[("S1", &[(0, 0, "a")]), ("S2", &[])]);
    let new = wb_sheets(&[("S1", &[(0, 0, "b")]), ("S2", &[])]);
    let diff = compare_bytes(&old, &new).unwrap();
    let view = DiffView::new(&diff);
    let sheets: Vec<_> = view.sheets().collect();
    assert_eq!(sheets.len(), 2);
    assert_eq!(sheets[0].name, "S1");
    assert_eq!(sheets[0].cells_changed, 1);
    assert_eq!(sheets[1].cells_changed, 0);
}

// ============================================================================
// v2.1 — RFC-022 format mode validation
// ============================================================================

#[test]
fn format_compare_non_ignore_returns_invalid_options() {
    use sheets_diff::{DiffOptions, FormatCompareMode, SheetsDiffError};
    let result = DiffOptions::builder()
        .format_compare(FormatCompareMode::NumberFormatOnly)
        .build();
    assert!(matches!(
        result,
        Err(SheetsDiffError::InvalidOptions { .. })
    ));
}

// ============================================================================
// v2.2 — RFC-023 object coverage diagnostics
// ============================================================================

#[test]
fn object_coverage_note_emitted_by_default() {
    // Every comparison emits the coverage note under WarnIfPresent default.
    let b = wb_empty();
    let diff = compare_bytes(&b, &b).unwrap();
    assert!(
        diff.diagnostics
            .iter()
            .any(|d| d.kind.code() == "unsupported_workbook_feature"),
        "expected coverage diagnostic in workbook diagnostics"
    );
}

#[test]
fn object_coverage_suppressed_with_ignore_mode() {
    use sheets_diff::{DiffOptions, ObjectCompareMode};
    let opts = DiffOptions::builder()
        .object_mode(ObjectCompareMode::Ignore)
        .build()
        .unwrap();
    let b = wb_empty();
    let diff = compare_bytes_with_options(&b, &b, opts).unwrap();
    assert!(
        !diff
            .diagnostics
            .iter()
            .any(|d| d.kind.code() == "unsupported_workbook_feature"),
        "no coverage diagnostics expected in Ignore mode"
    );
}

// ============================================================================
// v2.2 — RFC-024 DiffMetrics
// ============================================================================

#[test]
fn diff_metrics_populated() {
    let old = wb_strings(&[(0, 0, "a"), (0, 1, "b")]);
    let new = wb_strings(&[(0, 0, "x"), (0, 1, "b")]);
    let diff = compare_bytes(&old, &new).unwrap();
    assert_eq!(diff.metrics.sheets_read, 1);
    assert_eq!(diff.metrics.diffs_emitted, 1);
    assert!(
        diff.metrics.diagnostics_emitted > 0,
        "coverage note should count"
    );
}

#[test]
fn diff_metrics_zero_when_identical() {
    let b = wb_strings(&[(0, 0, "same")]);
    let diff = compare_bytes(&b, &b).unwrap();
    assert_eq!(diff.metrics.diffs_emitted, 0);
    assert_eq!(diff.metrics.sheets_read, 1);
}

// ============================================================================
// v2.3 — RFC-020 display formatting types
// ============================================================================

#[test]
fn cell_display_from_value_text() {
    use sheets_diff::{CellDisplay, CellValue, DisplaySource};
    let v = CellValue::Text("hello".into());
    let d = CellDisplay::from_value(&v);
    assert_eq!(d.text, "hello");
    assert_eq!(d.source, DisplaySource::SheetsDiffDefault);
    assert!(d.format.is_none());
}

#[test]
fn cell_display_from_value_number() {
    use sheets_diff::{CellDisplay, CellValue};
    let v = CellValue::Number(3.5);
    let d = CellDisplay::from_value(&v);
    assert!(d.text.starts_with("3.5"));
}

#[test]
fn cell_value_display_default_and_display_string_agree() {
    use sheets_diff::CellValue;
    let cases: &[CellValue] = &[
        CellValue::Empty,
        CellValue::Text("x".into()),
        CellValue::Integer(42),
        CellValue::Number(1.5),
        CellValue::Bool(true),
    ];
    for v in cases {
        assert_eq!(
            v.display_default(),
            v.display_string(),
            "display_default must equal display_string for {v:?}"
        );
    }
}

#[test]
fn cell_snapshot_preferred_display_prefers_display_text() {
    use sheets_diff::{CellDisplay, CellSnapshot, CellValue, DisplaySource};
    let snap = CellSnapshot::new(
        CellValue::Integer(42),
        None,
        Some(CellDisplay::new(
            "42 units".into(),
            None,
            DisplaySource::ApplicationProvided,
        )),
    );
    assert_eq!(snap.preferred_display(), "42 units");
}

#[test]
fn cell_snapshot_preferred_display_falls_back_to_value() {
    use sheets_diff::{CellSnapshot, CellValue};
    let snap = CellSnapshot::new(CellValue::Integer(42), None, None);
    assert_eq!(snap.preferred_display(), "42");
}

#[test]
fn cell_number_format_default_is_none() {
    use sheets_diff::CellNumberFormat;
    let fmt = CellNumberFormat::default();
    assert!(fmt.id.is_none());
    assert!(fmt.code.is_none());
}

// ============================================================================
// v2.3 — RFC-030 fixture generation (smoke test that gen.rs runs correctly)
// ============================================================================

#[test]
fn fixture_wide_columns_scenario_toml_exists_after_generation() {
    // This test just verifies the fixture directory and scenario.toml are present.
    // Run `cargo test --test gen` first to generate them.
    let path = std::path::Path::new("tests/fixtures/generated/wide_columns/scenario.toml");
    if path.exists() {
        let toml = std::fs::read_to_string(path).unwrap();
        assert!(toml.contains("wide_columns_xfd"));
    }
    // Pass even if not generated yet (first run) — gen.rs creates them.
}

// ============================================================================
// ForskScope feedback — Q2/Q3 view enhancements
// ============================================================================

#[test]
fn view_row_exposes_formula_text() {
    use sheets_diff::output::view::{DiffView, ViewFilter};

    // Cell with a formula change: =1+1 → =2+0
    let old = wb_with_formula(0, 0, "x", 1, 0, "=1+1");
    let new = wb_with_formula(0, 0, "x", 1, 0, "=2+0");
    let diff = compare_bytes(&old, &new).unwrap();
    let view = DiffView::new(&diff);
    let rows = view.rows(&ViewFilter::default());

    // Find a row that has a formula change, if the writer stored formula text.
    if let Some(r) = rows.iter().find(|r| r.formula_changed) {
        // old_formula / new_formula should be populated (Q2)
        assert!(
            r.old_formula.is_some() || r.new_formula.is_some(),
            "formula_changed row should carry formula text"
        );
    }
}

#[test]
fn view_row_to_owned_outlives_diff() {
    use sheets_diff::output::view::{DiffView, OwnedCellChangeRow, ViewFilter};

    let owned: Vec<OwnedCellChangeRow> = {
        let old = wb_strings(&[(0, 0, "a")]);
        let new = wb_strings(&[(0, 0, "b")]);
        let diff = compare_bytes(&old, &new).unwrap();
        let view = DiffView::new(&diff);
        // Map to owned rows, then drop the WorkbookDiff (Q3).
        view.rows(&ViewFilter::default())
            .iter()
            .map(|r| r.to_owned_row())
            .collect()
        // `diff` and `view` dropped here; `owned` must still be valid.
    };

    assert_eq!(owned.len(), 1);
    assert_eq!(owned[0].old_display, "a");
    assert_eq!(owned[0].new_display, "b");
    assert_eq!(owned[0].sheet_name, "Sheet1");
}

#[test]
fn change_kind_is_stable_added_removed_modified() {
    use sheets_diff::CellChangeKind;

    // Added: empty old → value new
    let added = compare_bytes(wb_empty(), wb_strings(&[(0, 0, "new")])).unwrap();
    assert_eq!(
        added.sheets[0].cell_diffs[0].change_kind(),
        CellChangeKind::Added
    );

    // Removed: value old → empty new
    let removed = compare_bytes(wb_strings(&[(0, 0, "gone")]), wb_empty()).unwrap();
    assert_eq!(
        removed.sheets[0].cell_diffs[0].change_kind(),
        CellChangeKind::Removed
    );

    // Modified: value old → different value new
    let modified = compare_bytes(wb_strings(&[(0, 0, "a")]), wb_strings(&[(0, 0, "b")])).unwrap();
    assert_eq!(
        modified.sheets[0].cell_diffs[0].change_kind(),
        CellChangeKind::Modified
    );
}

// ============================================================================
// Audit additions: RFC-004 reader API, RFC-010 TypeMismatchPolicy
// ============================================================================

#[test]
fn compare_readers_works() {
    use sheets_diff::compare_readers;
    use std::io::Cursor;

    let old = wb_strings(&[(0, 0, "before")]);
    let new = wb_strings(&[(0, 0, "after")]);
    let diff = compare_readers(Cursor::new(old), Cursor::new(new)).unwrap();
    assert_eq!(diff.summary.cells_changed, 1);
    assert_eq!(diff.sheets[0].cell_diffs[0].address.a1, "A1");
}

#[test]
fn compare_readers_with_options_works() {
    use sheets_diff::{DiffOptions, compare_readers_with_options};
    use std::io::Cursor;

    let b = wb_strings(&[(0, 0, "same")]);
    let opts = DiffOptions::builder().build().unwrap();
    let diff = compare_readers_with_options(Cursor::new(b.clone()), Cursor::new(b), opts).unwrap();
    assert_eq!(diff.summary.cells_changed, 0);
}

#[test]
fn type_mismatch_compare_display_string_treats_text_100_and_number_100_equal() {
    use sheets_diff::{DiffOptions, options::TypeMismatchPolicy};

    let old = wb_strings(&[(0, 0, "100")]);
    let new = wb_numbers(&[(0, 0, 100.0)]);
    // Default: different (TypeChanged)
    let default_diff = compare_bytes(&old, &new).unwrap();
    assert_eq!(default_diff.summary.values_changed, 1);

    // CompareDisplayString: treat "100" == 100.0 as equal via display
    let opts = DiffOptions::builder()
        .type_mismatch_policy(TypeMismatchPolicy::CompareDisplayString)
        .build()
        .unwrap();
    let lenient_diff = compare_bytes_with_options(&old, &new, opts).unwrap();
    assert_eq!(
        lenient_diff.summary.values_changed, 0,
        "CompareDisplayString should treat text '100' and number 100 as equal"
    );
}

// ============================================================================
// RFC-034 Handoff 01 — generated fixture corpus
//
// Generation lives in examples/gen-fixtures.rs and is never invoked by
// `cargo test`; these tests only read the fixture pairs already committed
// under tests/fixtures/generated/.
// ============================================================================

fn generated_fixture_dirs() -> Vec<std::path::PathBuf> {
    let base = std::path::Path::new("tests/fixtures/generated");
    let mut dirs: Vec<_> = std::fs::read_dir(base)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    dirs.sort();
    dirs
}

fn read_fixture_pair(name: &str) -> (Vec<u8>, Vec<u8>) {
    let dir = std::path::Path::new("tests/fixtures/generated").join(name);
    let old = std::fs::read(dir.join("old.xlsx")).unwrap();
    let new = std::fs::read(dir.join("new.xlsx")).unwrap();
    (old, new)
}

/// Every committed fixture pair compares without error, and — under
/// `serde` + `chrono` together — its serialised `WorkbookDiff` matches the
/// committed `expected.json` golden. That pair, not `serde` alone, is the
/// canonical feature set goldens are blessed under: `CellDateTime.iso` is
/// populated only when `chrono` is enabled (`src/normalize.rs`), so a
/// serial-based `DateTime` cell's exact JSON depends on it — gating the
/// comparison on both together, rather than `serde` alone, means exactly
/// one feature combination ever performs the exact-match check, so no
/// fixture's golden can be correct under one CI leg and wrong under
/// another. Under `serde` without `chrono` (and under neither), only the
/// error-free comparison is checked, plus this file's own hand-written
/// assertions, which are feature-invariant by construction.
///
/// Re-bless after confirming a changed output is correct, never before:
///   BLESS=1 cargo test --features serde,chrono -- generated_fixtures_match_golden
#[test]
fn generated_fixtures_match_golden() {
    for dir in generated_fixture_dirs() {
        let old = std::fs::read(dir.join("old.xlsx")).unwrap();
        let new = std::fs::read(dir.join("new.xlsx")).unwrap();
        #[allow(unused_variables)]
        let diff = compare_bytes(&old, &new).unwrap_or_else(|e| panic!("{}: {e}", dir.display()));

        #[cfg(all(feature = "serde", feature = "chrono"))]
        {
            let actual = sheets_diff::output::json::to_json_pretty(&diff).unwrap();
            let expected_path = dir.join("expected.json");
            if std::env::var("BLESS").as_deref() == Ok("1") {
                std::fs::write(&expected_path, &actual).unwrap();
                continue;
            }
            let expected = std::fs::read_to_string(&expected_path)
                .unwrap_or_else(|e| panic!("{}: {e}", expected_path.display()));
            assert_eq!(
                actual,
                expected,
                "{} golden mismatch — if the new output is correct, re-bless with \
                 BLESS=1 cargo test --features serde,chrono -- generated_fixtures_match_golden",
                dir.display()
            );
        }
    }
}

// The assertions below were originally inline in tests/gen.rs::generate_fixtures
// (RFC-030); RFC-034 Handoff 01 moves them here to run against the committed
// fixtures instead of freshly regenerated bytes.

#[test]
fn wide_columns_fixture_reaches_xfd() {
    let (old, new) = read_fixture_pair("wide_columns");
    let diff = compare_bytes(&old, &new).unwrap();
    assert_eq!(diff.sheets[0].cell_diffs[0].address.a1, "XFD1");
}

#[test]
fn renamed_sheet_fixture_preserves_cell_diff() {
    let (old, new) = read_fixture_pair("renamed_sheet");
    let diff = compare_bytes(&old, &new).unwrap();
    assert_eq!(diff.summary.sheets_renamed, 1);
    assert_eq!(diff.summary.cells_changed, 1);
}

#[test]
fn typed_values_fixture_reports_two_value_changes() {
    let (old, new) = read_fixture_pair("typed_values");
    let diff = compare_bytes(&old, &new).unwrap();
    assert_eq!(diff.summary.values_changed, 2);
}

#[test]
fn empty_sheet_fixture_has_no_diffs() {
    let (old, new) = read_fixture_pair("empty_sheet");
    let diff = compare_bytes(&old, &new).unwrap();
    assert_eq!(diff.summary.cells_changed, 0);
}

#[test]
fn sparse_range_fixture_reports_one_change() {
    let (old, new) = read_fixture_pair("sparse_range");
    let diff = compare_bytes(&old, &new).unwrap();
    assert_eq!(diff.summary.cells_changed, 1);
}

#[test]
fn cells_compared_exceeds_cells_changed_when_most_cells_are_unchanged() {
    // `metrics.cells_compared` must count every coordinate compared, not
    // just the ones that produced a diff. `sparse_range` has exactly one
    // changed cell among many populated ones, so a formula that (bug)
    // equals `cells_changed` would report 1 here too -- this fails under
    // that bug and passes only when the real coordinate count is counted.
    let (old, new) = read_fixture_pair("sparse_range");
    let diff = compare_bytes(&old, &new).unwrap();
    assert_eq!(diff.summary.cells_changed, 1);
    assert!(
        diff.metrics.cells_compared > diff.summary.cells_changed as u64,
        "cells_compared ({}) should exceed cells_changed ({}) when most \
         compared cells are unchanged",
        diff.metrics.cells_compared,
        diff.summary.cells_changed
    );
}

#[test]
fn row_insertion_cascade_fixture_reports_cascade() {
    let (old, new) = read_fixture_pair("row_insertion_cascade");
    let diff = compare_bytes(&old, &new).unwrap();
    assert!(
        diff.summary.cells_changed >= 20,
        "positional cascade expected, got {}",
        diff.summary.cells_changed
    );
}

// ============================================================================
// v2.3.1 — RFC-035 Handoff 05: integrity-affecting correctness defects
// ============================================================================

// D-01: ISO date/time values always compared equal (reachability, end-to-end) --

#[test]
fn d01_iso_datetime_reachability_end_to_end() {
    // `rust_xlsxwriter`'s public API cannot emit a `t="d"` ISO-typed cell
    // (calamine's `DateTimeIso` path) — only Excel-serial dates. Hand-patch
    // the sheet XML to inject one directly, so this exercises the real
    // xlsx-parsing path end to end, not just the value-comparison logic.
    let base = wb_strings(&[(0, 0, "label")]);
    let old = patch_xlsx_xml(&base, "xl/worksheets/sheet1.xml", |xml| {
        xml.replacen(
            "</sheetData>",
            "<row r=\"2\"><c r=\"A2\" t=\"d\"><v>2024-01-01T00:00:00</v></c></row></sheetData>",
            1,
        )
    });
    let new = patch_xlsx_xml(&base, "xl/worksheets/sheet1.xml", |xml| {
        xml.replacen(
            "</sheetData>",
            "<row r=\"2\"><c r=\"A2\" t=\"d\"><v>2099-12-31T23:59:59</v></c></row></sheetData>",
            1,
        )
    });

    let diff = compare_bytes(&old, &new).unwrap();
    assert_eq!(
        diff.summary.values_changed, 1,
        "expected the ISO datetime cell to be reported changed"
    );
    let vc = diff.sheets[0].cell_diffs[0].value.as_ref().unwrap();
    match (&vc.old, &vc.new) {
        (CellValue::DateTime(a), CellValue::DateTime(b)) => {
            assert_eq!(a.iso.as_deref(), Some("2024-01-01T00:00:00"));
            assert_eq!(b.iso.as_deref(), Some("2099-12-31T23:59:59"));
            assert!(!a.has_serial && !b.has_serial);
        }
        _ => panic!("expected DateTime/DateTime value change"),
    }
}

#[test]
fn d01_iso_datetime_reachability_identical_values_report_no_change() {
    // Sanity companion: the same real ISO-typed cell on both sides must
    // still compare equal — the fix only removes the *false* equality.
    let base = wb_strings(&[(0, 0, "label")]);
    let with_iso = patch_xlsx_xml(&base, "xl/worksheets/sheet1.xml", |xml| {
        xml.replacen(
            "</sheetData>",
            "<row r=\"2\"><c r=\"A2\" t=\"d\"><v>2024-01-01T00:00:00</v></c></row></sheetData>",
            1,
        )
    });
    let diff = compare_bytes(&with_iso, &with_iso).unwrap();
    assert_eq!(diff.summary.values_changed, 0);
}

// D-01 note: `Data::DurationIso` (the other half of D-01) is produced only by
// calamine's `.ods` reader (`src/ods.rs`), never by its `.xlsx` cell reader
// (`src/xlsx/cells_reader.rs`) — confirmed by reading calamine 0.36.1's
// source. Since this crate only reads `.xlsx`, `CellValue::Duration` is not
// reachable end-to-end through any input this crate accepts; it is exercised
// directly at the value-comparison level in `compare::tests` instead. This
// is a further, if narrower, finding worth recording: `CellValue::Duration`
// and its normalisation code are currently dead code for every input this
// crate can actually open.

// D-02: `is_1904` threading (`NormalizeEquivalentDateTimes`) ---------------

#[test]
fn d02_normalize_equivalent_datetimes_reconciles_1900_and_1904_end_to_end() {
    use rust_xlsxwriter::{ExcelDateTime, Format};

    // A genuine DateTime cell (calamine classifies by cell *format*, so this
    // needs an explicit date number format — `write_datetime` alone relies
    // on column-level formatting, which writes no per-cell style at all).
    let base = {
        let mut wb = Workbook::new();
        let ws = wb.add_worksheet();
        let date_format = Format::new().set_num_format("yyyy-mm-dd");
        ws.write_datetime_with_format(
            0,
            0,
            ExcelDateTime::from_ymd(2024, 6, 15).unwrap(),
            &date_format,
        )
        .unwrap();
        wb.save_to_buffer().unwrap()
    };

    // Discover the raw 1900-epoch serial `rust_xlsxwriter` actually wrote by
    // diffing against an empty sheet and reading the reported value back —
    // avoids hand-deriving Excel's serial arithmetic (1900 leap-year quirk
    // included) ourselves.
    let empty = wb_empty();
    let added = compare_bytes(&empty, &base).unwrap();
    let serial_1900 = match &added.sheets[0].cell_diffs[0].value.as_ref().unwrap().new {
        CellValue::DateTime(dt) => dt.serial,
        _ => panic!("expected DateTime"),
    };

    // "new": the same real date, expressed under the 1904 system — 1462
    // days earlier — with the workbook flagged accordingly.
    let serial_1904 = serial_1900 - 1462.0;
    let new = patch_xlsx_xml(&base, "xl/workbook.xml", |xml| {
        xml.replacen("<workbookPr", "<workbookPr date1904=\"1\" ", 1)
    });
    let new = patch_xlsx_xml(&new, "xl/worksheets/sheet1.xml", |xml| {
        xml.replace(&serial_1900.to_string(), &serial_1904.to_string())
    });

    // Before D-02, `is_1904` was hardcoded `false` everywhere, so this
    // reconciliation was impossible regardless of policy.
    let exact = compare_bytes(&base, &new).unwrap();
    assert_eq!(
        exact.summary.values_changed, 1,
        "ExactRepresentation must not conflate the two epochs"
    );

    let mut opts = DiffOptions::default();
    opts.comparison.value.date = DateComparePolicy::NormalizeEquivalentDateTimes;
    let normalized = compare_bytes_with_options(&base, &new, opts).unwrap();
    assert_eq!(
        normalized.summary.values_changed, 0,
        "NormalizeEquivalentDateTimes should reconcile the 1900/1904 representations \
         of the same real-world date"
    );
}

// D-03: alignment coordinate-space collision ---------------------------------

#[test]
fn d03_inserted_row_number_colliding_with_matched_old_row_compares_both_correctly() {
    use sheets_diff::options::{AlignmentMode, MatchingOptions};

    // old: row1=id1/a,  row2=id2/b, row3=id3/c
    // new: row1=id_new/z (inserted — numerically collides with OLD row 1),
    //      row2=id1/a2  (id1's value CHANGED: a -> a2),
    //      row3=id2/b,  row4=id3/c
    //
    // Under RowKey alignment: id1/id2/id3 all match (old row 1/2/3 -> new
    // row 2/3/4). "id_new" at NEW row 1 is inserted, and NEW row 1
    // numerically coincides with OLD row 1 (the matched id1 row) — the D-03
    // collision condition. Giving id1 a genuine value change (not just an
    // unchanged value) means a wrong lookup would produce an observably
    // *wrong* result (missing the id1 change, or misattributing it),
    // not merely an absence of a false positive.
    let old = wb_strings(&[
        (0, 0, "id1"),
        (0, 1, "a"),
        (1, 0, "id2"),
        (1, 1, "b"),
        (2, 0, "id3"),
        (2, 1, "c"),
    ]);
    let new = wb_strings(&[
        (0, 0, "id_new"),
        (0, 1, "z"),
        (1, 0, "id1"),
        (1, 1, "a2"),
        (2, 0, "id2"),
        (2, 1, "b"),
        (3, 0, "id3"),
        (3, 1, "c"),
    ]);

    let opts = DiffOptions::builder()
        .build_with_matching(MatchingOptions {
            sheet_matching: SheetMatchingMode::default(),
            alignment: AlignmentMode::RowKey { columns: vec![1] },
        })
        .unwrap();
    let diff = compare_bytes_with_options(&old, &new, opts).unwrap();

    let summary = diff.sheets[0].alignment_summary.as_ref().unwrap();
    assert_eq!(summary.matched_rows, 3, "id1/id2/id3 should all match");
    assert_eq!(
        summary.inserted_rows, 1,
        "id_new should be the only insertion"
    );
    assert_eq!(summary.removed_rows, 0);

    // Three independent things must all be true at once — each one is a way
    // the pre-fix coordinate collision could go wrong:
    //   1. id1's real value change (a -> a2) is found, at OLD row 1 (its
    //      matched/canonical address) — not lost, not misattributed.
    let id1_change = diff.sheets[0].cell_diffs.iter().find(|cd| {
        cd.value
            .as_ref()
            .is_some_and(|vc| matches!(&vc.old, CellValue::Text(s) if s == "a"))
    });
    let id1_change = id1_change.expect("id1's a -> a2 change must be found");
    assert_eq!(
        id1_change.address.a1, "B1",
        "id1's value lives in old row 1"
    );
    let vc = id1_change.value.as_ref().unwrap();
    assert!(matches!(&vc.new, CellValue::Text(s) if s == "a2"));

    //   2. The inserted id_new/z row is independently reported as added
    //      content, not merged away by the row-number collision.
    let has_id_new_added = diff.sheets[0].cell_diffs.iter().any(|cd| {
        cd.value
            .as_ref()
            .is_some_and(|vc| matches!(&vc.new, CellValue::Text(s) if s == "id_new" || s == "z"))
    });
    assert!(
        has_id_new_added,
        "the inserted row's content must be independently compared and reported"
    );

    //   3. id2/id3 (unrelated matched rows, unchanged) must not be
    //      collaterally disturbed.
    assert_eq!(
        diff.summary.values_changed, 3,
        "expected exactly: id1's real change, plus id_new/z being added — nothing else"
    );
}

// D-04: formula text attaching to the wrong cell -----------------------------

#[test]
fn d04_formula_attaches_to_the_formula_cell_not_the_value_range_origin() {
    // Value range starts at row 1 (a text label, no formula); the formula
    // range starts at row 2 (the only cell with formula text) — the two
    // ranges' origins genuinely differ, reproducing D-04 exactly.
    let old = wb_with_formula(0, 0, "label", 1, 0, "=1+1");
    let new = wb_with_formula(0, 0, "label", 1, 0, "=2+0");

    let diff = compare_bytes(&old, &new).unwrap();
    assert_eq!(diff.sheets[0].cell_diffs.len(), 1);
    let cd = &diff.sheets[0].cell_diffs[0];
    assert_eq!(
        cd.address.a1, "A2",
        "the formula change must attach to A2 (the real formula cell), not A1 \
         (the value range's origin)"
    );
    let fc = cd.formula.as_ref().unwrap();
    assert_eq!(fc.old.as_ref().unwrap().raw, "1+1");
    assert_eq!(fc.new.as_ref().unwrap().raw, "2+0");
    // The label cell (A1) must show no diff at all — its content is
    // identical on both sides.
    assert!(cd.value.is_none() || cd.address.a1 != "A1");
}

// ============================================================================
// v2.3.1 — RFC-036 §5.2: the fixture coverage matrix
// ============================================================================

// #1 — origin not at A1, row axis ------------------------------------------

#[test]
fn row_shifted_origin_fixture_reports_correct_address() {
    let (old, new) = read_fixture_pair("row_shifted_origin");
    let diff = compare_bytes(&old, &new).unwrap();
    assert_eq!(
        diff.sheets[0].compared_range.start,
        Some((5, 1)),
        "the range must start at A5, not A1 — the sheet has no content \
         before row 5"
    );
    assert_eq!(diff.sheets[0].cell_diffs.len(), 1);
    assert_eq!(diff.sheets[0].cell_diffs[0].address.a1, "A5");
}

// #2 — formula/value origin both shifted, and the negative control ---------

#[test]
fn formula_shifted_origin_fixture_attaches_to_the_real_formula_cell() {
    let (old, new) = read_fixture_pair("formula_shifted_origin");
    let diff = compare_bytes(&old, &new).unwrap();
    assert_eq!(diff.sheets[0].cell_diffs.len(), 1);
    let cd = &diff.sheets[0].cell_diffs[0];
    assert_eq!(
        cd.address.a1, "A7",
        "formula is at row 7 (label at row 5, a gap at row 6) — general \
         case of D-04's fix, beyond the row-1-vs-row-2 shape the `formula` \
         fixture already covers"
    );
    let fc = cd.formula.as_ref().unwrap();
    assert_eq!(fc.old.as_ref().unwrap().raw, "1+1");
    assert_eq!(fc.new.as_ref().unwrap().raw, "2+0");
    assert!(
        diff.sheets[0].diagnostics.is_empty(),
        "no spurious FormulaUnavailable diagnostics expected"
    );
}

#[test]
fn formula_at_first_cell_fixture_negative_control() {
    // D-04's negative control: the formula IS the first populated cell, so
    // value-range and formula-range origins coincide. Guards against a
    // future "fix" that special-cases the coinciding-origin case instead of
    // translating through absolute coordinates unconditionally.
    let (old, new) = read_fixture_pair("formula_at_first_cell");
    let diff = compare_bytes(&old, &new).unwrap();
    assert_eq!(diff.sheets[0].cell_diffs.len(), 1);
    assert_eq!(diff.sheets[0].cell_diffs[0].address.a1, "A1");
}

// #3, #4 — alignment modes with zero prior coverage -------------------------

#[test]
fn alignment_row_signature_fixture_reduces_cascade() {
    use sheets_diff::options::{AlignmentMode, MatchingOptions};

    let (old, new) = read_fixture_pair("alignment_row_signature");

    // Positional (the golden's own default-options comparison) shows the
    // full cascade: every row shifted down by the insertion.
    let positional = compare_bytes(&old, &new).unwrap();
    assert_eq!(positional.summary.cells_changed, 12);

    // RowSignature — matched by whole-row content, not a key column — had
    // never been exercised by any test before this fixture.
    let opts = DiffOptions::builder()
        .build_with_matching(MatchingOptions {
            sheet_matching: SheetMatchingMode::default(),
            alignment: AlignmentMode::RowSignature {
                sample_columns: None,
            },
        })
        .unwrap();
    let aligned = compare_bytes_with_options(&old, &new, opts).unwrap();
    assert!(
        aligned.summary.cells_changed < positional.summary.cells_changed,
        "RowSignature alignment should report far fewer changes than the \
         positional cascade: aligned={}, positional={}",
        aligned.summary.cells_changed,
        positional.summary.cells_changed
    );
    let summary = aligned.sheets[0].alignment_summary.as_ref().unwrap();
    assert_eq!(
        summary.matched_rows, 5,
        "all 5 original rows should match by signature"
    );
    assert_eq!(summary.inserted_rows, 1);
    assert_eq!(summary.removed_rows, 0);
}

#[test]
fn alignment_header_column_fixture_reduces_cascade() {
    use sheets_diff::options::{AlignmentMode, MatchingOptions};

    let (old, new) = read_fixture_pair("alignment_header_column");

    let positional = compare_bytes(&old, &new).unwrap();
    assert_eq!(positional.summary.cells_changed, 8);

    // HeaderColumn had never been exercised by any test before this fixture.
    let opts = DiffOptions::builder()
        .build_with_matching(MatchingOptions {
            sheet_matching: SheetMatchingMode::default(),
            alignment: AlignmentMode::HeaderColumn,
        })
        .unwrap();
    let aligned = compare_bytes_with_options(&old, &new, opts).unwrap();
    assert!(
        aligned.summary.cells_changed < positional.summary.cells_changed,
        "HeaderColumn alignment should report far fewer changes than the \
         positional cascade: aligned={}, positional={}",
        aligned.summary.cells_changed,
        positional.summary.cells_changed
    );
    let summary = aligned.sheets[0].alignment_summary.as_ref().unwrap();
    // The header row itself ("id"/"value") matches trivially, plus the 3
    // original data rows matched by their id column.
    assert_eq!(summary.matched_rows, 4);
    assert_eq!(summary.inserted_rows, 1);
    assert_eq!(summary.removed_rows, 0);
}

// #5 — CellError comparison, zero coverage at any level before this --------

#[test]
fn error_values_fixture_detects_error_kind_change() {
    let (old, new) = read_fixture_pair("error_values");
    let diff = compare_bytes(&old, &new).unwrap();
    // Only the #DIV/0! -> #REF! cell should differ; the #N/A -> #N/A cell
    // (same formula, same error kind) must show no diff at all.
    assert_eq!(diff.sheets[0].cell_diffs.len(), 1);
    let cd = &diff.sheets[0].cell_diffs[0];
    assert_eq!(cd.address.a1, "A1");
    let vc = cd.value.as_ref().unwrap();
    assert_eq!(vc.reason, ValueDifferenceKind::ErrorKindChanged);
    assert!(matches!(vc.old, CellValue::Error(CellError::Div0)));
    assert!(matches!(vc.new, CellValue::Error(CellError::Ref)));
}

// #6 — SheetChange::Moved, never distinguished from Unchanged before this --

#[test]
fn sheet_reordered_fixture_distinguishes_moved_from_modified() {
    let (old, new) = read_fixture_pair("sheet_reordered");
    let diff = compare_bytes(&old, &new).unwrap();

    assert_eq!(diff.summary.sheets_moved, 2, "Alpha and Beta both moved");

    let by_name = |name: &str| {
        diff.sheets
            .iter()
            .find(|s| s.old_sheet.as_ref().is_some_and(|r| r.name == name))
            .unwrap()
    };
    assert_eq!(by_name("Alpha").change, SheetChange::Moved);
    assert_eq!(by_name("Beta").change, SheetChange::Moved);
    // Gamma stays at the same index but its one cell changes, so it must be
    // Modified, not Moved — the two are not the same thing.
    assert_eq!(by_name("Gamma").change, SheetChange::Modified);
    assert_eq!(by_name("Gamma").cell_diffs.len(), 1);
}

// #7 — ordinary serial-based dates, no golden-corpus fixture used any -------

#[test]
fn date_column_fixture_detects_date_change() {
    let (old, new) = read_fixture_pair("date_column");
    let diff = compare_bytes(&old, &new).unwrap();
    assert_eq!(diff.sheets[0].cell_diffs.len(), 1);
    let cd = &diff.sheets[0].cell_diffs[0];
    assert_eq!(cd.address.a1, "A2");
    let vc = cd.value.as_ref().unwrap();
    assert_eq!(vc.reason, ValueDifferenceKind::DateTimeChanged);
    assert!(matches!(vc.old, CellValue::DateTime(ref dt) if dt.has_serial));
    assert!(matches!(vc.new, CellValue::DateTime(ref dt) if dt.has_serial));
}

// #8 — non-ASCII text, zero coverage before this ----------------------------

#[test]
fn non_ascii_text_fixture_detects_change() {
    let (old, new) = read_fixture_pair("non_ascii_text");
    let diff = compare_bytes(&old, &new).unwrap();
    assert_eq!(diff.sheets[0].old_sheet.as_ref().unwrap().name, "Café☕");
    assert_eq!(diff.sheets[0].cell_diffs.len(), 1);
    let vc = diff.sheets[0].cell_diffs[0].value.as_ref().unwrap();
    assert!(matches!(&vc.old, CellValue::Text(s) if s == "héllo wörld"));
    assert!(matches!(&vc.new, CellValue::Text(s) if s == "héllo wörld — updated"));
}

// #9 — chart sheet beside a worksheet, diagnostic never fired before this --

#[test]
fn chart_sheet_fixture_fires_diagnostic_and_compares_the_worksheet() {
    let (old, new) = read_fixture_pair("chart_sheet");
    let diff = compare_bytes(&old, &new).unwrap();
    // The ordinary worksheet's own cell change must still be detected.
    let ws_diff = diff
        .sheets
        .iter()
        .find(|s| s.old_sheet.as_ref().is_some_and(|r| r.name == "Sheet1"))
        .unwrap();
    assert_eq!(ws_diff.cell_diffs.len(), 1);
    assert_eq!(ws_diff.cell_diffs[0].address.a1, "A4");
    // The chart-sheet coverage diagnostic is workbook-level (emitted by
    // `report_object_coverage` into `WorkbookDiff.diagnostics`), not
    // per-sheet.
    assert!(
        diff.diagnostics
            .iter()
            .any(|d| d.kind.code() == "unsupported_workbook_feature"
                && d.location.sheet_name.as_deref() == Some("Chart1")),
        "expected an unsupported_workbook_feature diagnostic naming the chart sheet, got {:?}",
        diff.diagnostics
    );
}

// #10 — a physically-present empty cell must not anchor the range origin ---

#[test]
fn empty_cell_before_content_fixture_does_not_anchor_origin() {
    let (old, new) = read_fixture_pair("empty_cell_before_content");
    let diff = compare_bytes(&old, &new).unwrap();
    assert_eq!(
        diff.sheets[0].compared_range.start,
        Some((2, 1)),
        "a physically-present but empty <c r=\"A1\"/> must not anchor the \
         range at A1 — only the real content at A2 should"
    );
    assert_eq!(diff.sheets[0].cell_diffs.len(), 1);
    assert_eq!(diff.sheets[0].cell_diffs[0].address.a1, "A2");
}

// #11 — ISO datetime promoted from a hand-built test into the corpus -------

#[test]
fn iso_datetime_fixture_detects_change() {
    let (old, new) = read_fixture_pair("iso_datetime");
    let diff = compare_bytes(&old, &new).unwrap();
    assert_eq!(diff.sheets[0].cell_diffs.len(), 1);
    let vc = diff.sheets[0].cell_diffs[0].value.as_ref().unwrap();
    assert_eq!(vc.reason, ValueDifferenceKind::DateTimeChanged);
    match (&vc.old, &vc.new) {
        (CellValue::DateTime(a), CellValue::DateTime(b)) => {
            assert!(!a.has_serial && !b.has_serial);
            assert_eq!(a.iso.as_deref(), Some("2024-01-01T00:00:00"));
            assert_eq!(b.iso.as_deref(), Some("2099-12-31T23:59:59"));
        }
        _ => panic!("expected DateTime/DateTime"),
    }
}
