//! `AlignmentMode::RowKey` must not lose a row that has no key (f130).
//!
//! Through 3.0.0, `extract_row_keys` put a row in its map only if the row had a cell in a key
//! column. A row whose key cell was blank was in none of `matched`, `removed` or `inserted`, so it
//! was not compared at all, and the result said `confidence: Exact` about a sheet it had not
//! looked at. A change in such a row was reported as no change.
//!
//! **The rule now: a row with no cell in any key column is never absent.** Rows with the same
//! values in the same columns on both sides are paired with one another, in row order, and
//! compared like any matched pair, so an unchanged subtotal line is silent; the rest are reported
//! as removed (old side) or inserted (new side), so a changed keyless row reaches the comparison
//! as a whole-row change. A `missing_alignment_key` warning names the count on each side, and the
//! sheet's confidence is at most `Medium`. Pairing a *changed* keyless row with its counterpart
//! would be better than removed + inserted, and it is a separate design.
//!
//! The pairing exists because the first version of this fix reported every keyless row as removed
//! and inserted: 2,200 cell diffs for one changed cell on a 2,000-row sheet with a blank key every
//! twentieth row, and a workbook compared with itself reported differences.
//!
//! Written against the *codes* and the public result fields, so the tests that do not name the new
//! `DiagnosticKind` variant compile against the 3.0.0 source as well.

mod support;
use support::wb_strings;

use sheets_diff::options::AlignmentMode;
use sheets_diff::{
    DiagnosticKind, DiffOptions, MatchConfidence, Severity, SheetDiff, WorkbookDiff,
    compare_bytes_with_options,
};

const CODE: &str = "missing_alignment_key";

/// Four rows, `A` is the ID column and `B` a value. Row 3 has **no ID**: a subtotal / note line.
/// `note` is what row 3's `B` says.
fn sheet_with_blank_key_row(note: &str) -> Vec<u8> {
    wb_strings(&[
        (0, 0, "id1"),
        (0, 1, "v1"),
        (1, 0, "id2"),
        (1, 1, "v2"),
        (2, 1, note), // no cell in column A
        (3, 0, "id4"),
        (3, 1, "v4"),
    ])
}

fn opts(mode: AlignmentMode) -> DiffOptions {
    DiffOptions::builder().alignment(mode).build().unwrap()
}

fn row_key(columns: Vec<u32>) -> DiffOptions {
    opts(AlignmentMode::RowKey { columns })
}

fn diff(old: Vec<u8>, new: Vec<u8>, o: DiffOptions) -> WorkbookDiff {
    compare_bytes_with_options(old, new, o).unwrap()
}

fn only_sheet(d: &WorkbookDiff) -> &SheetDiff {
    assert_eq!(d.sheets.len(), 1);
    &d.sheets[0]
}

fn a1s(s: &SheetDiff) -> Vec<String> {
    s.cell_diffs.iter().map(|c| c.address.a1.clone()).collect()
}

fn warnings(s: &SheetDiff) -> Vec<&sheets_diff::Diagnostic> {
    s.diagnostics
        .iter()
        .filter(|d| d.kind.code() == CODE)
        .collect()
}

// ---------------------------------------------------------------------------
// The defect, and its control
// ---------------------------------------------------------------------------

/// ForskScope's case: one cell changed in a row with a blank key, `RowKey` on that column.
#[test]
fn a_change_in_a_blank_key_row_is_reported_under_rowkey() {
    let d = diff(
        sheet_with_blank_key_row("note-old"),
        sheet_with_blank_key_row("note-new"),
        row_key(vec![1]),
    );
    let s = only_sheet(&d);
    assert!(
        !s.cell_diffs.is_empty(),
        "the changed cell in the blank-key row was not compared (cell_diffs is empty; \
         alignment_summary = {:?})",
        s.alignment_summary
    );
    assert!(a1s(s).iter().all(|a| a == "B3"), "{:?}", a1s(s));
    // Both sides of the row are seen: the old text leaves, the new text arrives.
    let text: Vec<String> = s
        .cell_diffs
        .iter()
        .filter_map(|c| c.value.as_ref())
        .flat_map(|v| [format!("{:?}", v.old), format!("{:?}", v.new)])
        .collect();
    assert!(text.iter().any(|t| t.contains("note-old")), "{text:?}");
    assert!(text.iter().any(|t| t.contains("note-new")), "{text:?}");
}

/// The control: the same workbooks under `Positional` report the same change, so the fixture is
/// sound and the difference above is the alignment's doing.
#[test]
fn the_same_workbooks_under_positional_report_the_change() {
    let d = diff(
        sheet_with_blank_key_row("note-old"),
        sheet_with_blank_key_row("note-new"),
        opts(AlignmentMode::Positional),
    );
    assert_eq!(a1s(only_sheet(&d)), vec!["B3"]);
}

/// A blank-key row that changed must not be a *silent* absence: it cannot be paired with an
/// identical row, so it is unmatched — in `removed` and `inserted`, and counted by the summary.
#[test]
fn a_changed_keyless_row_is_counted_as_removed_and_inserted_not_absent() {
    let d = diff(
        sheet_with_blank_key_row("old-note"),
        sheet_with_blank_key_row("new-note"),
        row_key(vec![1]),
    );
    let a = only_sheet(&d).alignment_summary.as_ref().unwrap();
    assert_eq!(
        (a.matched_rows, a.removed_rows, a.inserted_rows),
        (3, 1, 1),
        "three keyed rows matched; the one keyless row on each side unmatched, not dropped"
    );
}

// ---------------------------------------------------------------------------
// The warning
// ---------------------------------------------------------------------------

#[test]
fn the_warning_fires_with_the_count_on_each_side() {
    // Old has two keyless rows (3 and 5); new has one (3).
    let old = wb_strings(&[
        (0, 0, "id1"),
        (0, 1, "v1"),
        (1, 1, "spacer-a"),
        (2, 0, "id3"),
        (2, 1, "v3"),
        (3, 1, "spacer-b"),
    ]);
    let new = wb_strings(&[
        (0, 0, "id1"),
        (0, 1, "v1"),
        (1, 1, "spacer-a"),
        (2, 0, "id3"),
        (2, 1, "v3"),
    ]);
    let d = diff(old, new, row_key(vec![1]));
    let s = only_sheet(&d);
    let w = warnings(s);
    assert_eq!(
        w.len(),
        1,
        "exactly one warning per sheet: {:?}",
        s.diagnostics
    );
    assert_eq!(w[0].severity, Severity::Warning);
    assert_eq!(
        w[0].kind,
        DiagnosticKind::MissingAlignmentKey {
            old_count: 2,
            new_count: 1
        }
    );
    // It names its sheet, by order and by name, like the other two alignment warnings.
    assert_eq!(w[0].location.sheet_order, Some(0));
    assert_eq!(w[0].location.sheet_name.as_deref(), Some("Sheet1"));
    assert!(w[0].location.address.is_none());
    // The workbook-level list does not carry it: it is about a sheet.
    assert!(d.diagnostics.iter().all(|x| x.kind.code() != CODE));
    // The counters follow what was kept.
    assert_eq!(d.summary.diagnostics.warnings, 1);
}

#[test]
fn a_small_count_still_warns() {
    let d = diff(
        sheet_with_blank_key_row("x"),
        sheet_with_blank_key_row("x"),
        row_key(vec![1]),
    );
    let w = warnings(only_sheet(&d));
    assert_eq!(w.len(), 1);
    assert_eq!(
        w[0].kind,
        DiagnosticKind::MissingAlignmentKey {
            old_count: 1,
            new_count: 1
        }
    );
}

// ---------------------------------------------------------------------------
// Confidence
// ---------------------------------------------------------------------------

/// `Exact` is a claim that the rows were matched exactly. With a keyless row it is false.
/// The value chosen is `Medium`: the keyed rows are matched soundly (so not `Low`), but the sheet
/// has rows the alignment could not place (so not `High`, which says the pairing is reliable
/// apart from a few real insertions and removals).
#[test]
fn confidence_is_not_exact_when_a_row_was_keyless() {
    let d = diff(
        sheet_with_blank_key_row("same"),
        sheet_with_blank_key_row("same"),
        row_key(vec![1]),
    );
    let c = only_sheet(&d)
        .alignment_summary
        .as_ref()
        .unwrap()
        .confidence;
    assert_ne!(c, MatchConfidence::Exact);
    assert_eq!(c, MatchConfidence::Medium);
}

/// Even with the ratio ForskScope measured (a few percent keyless, most rows matched) the
/// confidence stays below `High`, and below `Exact` when every keyless row paired: both would be
/// computed from the counts alone.
#[test]
fn a_mostly_keyed_sheet_is_still_not_high() {
    let mut old = Vec::new();
    let ids: Vec<String> = (0..40).map(|i| format!("id{i}")).collect();
    for (i, id) in ids.iter().enumerate() {
        old.push((i as u32, 0u16, id.as_str()));
        old.push((i as u32, 1u16, "v"));
    }
    // One extra row far below, keyless.
    old.push((50, 1, "note"));
    let bytes = wb_strings(&old);
    let d = diff(bytes.clone(), bytes, row_key(vec![1]));
    let a = only_sheet(&d).alignment_summary.as_ref().unwrap();
    // 40 keyed rows and the one keyless row, which is identical on both sides and so pairs with itself.
    assert_eq!(
        (a.matched_rows, a.removed_rows, a.inserted_rows),
        (41, 0, 0)
    );
    // By the counts alone this is `Exact`. It is not: a row was placed by its content, not by a key.
    assert_eq!(a.confidence, MatchConfidence::Medium);
}

// ---------------------------------------------------------------------------
// The guard on Non-change scope: a fully keyed sheet is unaffected
// ---------------------------------------------------------------------------

#[test]
fn a_fully_keyed_sheet_has_no_warning_and_stays_exact() {
    let keyed = |v: &str| {
        wb_strings(&[
            (0, 0, "id1"),
            (0, 1, "v1"),
            (1, 0, "id2"),
            (1, 1, v),
            (2, 0, "id3"),
            (2, 1, "v3"),
        ])
    };
    let d = diff(keyed("v2"), keyed("changed"), row_key(vec![1]));
    let s = only_sheet(&d);
    assert!(warnings(s).is_empty(), "{:?}", s.diagnostics);
    let a = s.alignment_summary.as_ref().unwrap();
    assert_eq!(
        (
            a.matched_rows,
            a.removed_rows,
            a.inserted_rows,
            a.confidence
        ),
        (3, 0, 0, MatchConfidence::Exact)
    );
    assert_eq!(a1s(s), vec!["B2"]);
}

/// A genuine keyed insertion still aligns: the LCS over keyed rows is not what this fixes.
#[test]
fn keyed_insertion_still_aligns_as_before() {
    let old = wb_strings(&[(0, 0, "a"), (1, 0, "b"), (2, 0, "c")]);
    let new = wb_strings(&[(0, 0, "a"), (1, 0, "X"), (2, 0, "b"), (3, 0, "c")]);
    let d = diff(old, new, row_key(vec![1]));
    let s = only_sheet(&d);
    assert!(warnings(s).is_empty());
    let a = s.alignment_summary.as_ref().unwrap();
    assert_eq!((a.matched_rows, a.removed_rows, a.inserted_rows), (3, 0, 1));
    assert_eq!(a.confidence, MatchConfidence::High);
}

// ---------------------------------------------------------------------------
// Key columns that select nothing: the same defect, larger
// ---------------------------------------------------------------------------

/// A key column that no row has a cell in makes *every* row keyless. Through 3.0.0 that compared
/// nothing at all and called it `Exact`.
#[test]
fn a_key_column_no_row_populates_compares_everything_and_warns() {
    let d = diff(
        wb_strings(&[(0, 0, "a"), (1, 0, "b")]),
        wb_strings(&[(0, 0, "a"), (1, 0, "CHANGED")]),
        row_key(vec![9]),
    );
    let s = only_sheet(&d);
    assert!(!s.cell_diffs.is_empty(), "nothing was compared");
    assert_eq!(warnings(s).len(), 1);
    let a = s.alignment_summary.as_ref().unwrap();
    // Row 1 (`a`) is identical on both sides and pairs with itself; row 2 changed and does not.
    assert_eq!((a.matched_rows, a.removed_rows, a.inserted_rows), (1, 1, 1));
    assert_ne!(a.confidence, MatchConfidence::Exact);
}

#[test]
fn an_empty_key_column_list_compares_everything_and_warns() {
    let d = diff(
        wb_strings(&[(0, 0, "a"), (1, 0, "b")]),
        wb_strings(&[(0, 0, "a"), (1, 0, "CHANGED")]),
        row_key(vec![]),
    );
    let s = only_sheet(&d);
    assert!(!s.cell_diffs.is_empty(), "nothing was compared");
    assert_eq!(warnings(s).len(), 1);
}

// ---------------------------------------------------------------------------
// Identical keyless rows pair; a changed one does not
// ---------------------------------------------------------------------------

/// ForskScope's shape at small scale: unique IDs in column A, blank in every fifth row (a subtotal line).
fn sheet_with_periodic_blank_keys(rows: u32, cols: u16, change: Option<(u32, u16)>) -> Vec<u8> {
    let mut cells: Vec<(u32, u16, String)> = Vec::new();
    for r in 0..rows {
        if r % 5 != 4 {
            cells.push((r, 0, format!("id{r:04}")));
        }
        for c in 1..cols {
            let mut v = format!("r{r}c{c}");
            if change == Some((r, c)) {
                v.push_str("-CHANGED");
            }
            cells.push((r, c, v));
        }
    }
    let refs: Vec<(u32, u16, &str)> = cells.iter().map(|(r, c, v)| (*r, *c, v.as_str())).collect();
    wb_strings(&refs)
}

/// The reason for pairing: without it a workbook compared with itself reported a removal and an
/// insertion for every blank-key row. Now it reports nothing, and still says the alignment had a
/// keyless row in it.
#[test]
fn a_workbook_compared_with_itself_is_unchanged_under_rowkey_even_with_blank_keys() {
    let b = sheet_with_periodic_blank_keys(20, 4, None);
    let d = diff(b.clone(), b, row_key(vec![1]));
    let s = only_sheet(&d);
    assert!(s.cell_diffs.is_empty(), "{:?}", a1s(s));
    assert_eq!(format!("{:?}", s.change), "Unchanged");
    let a = s.alignment_summary.as_ref().unwrap();
    assert_eq!(
        (a.matched_rows, a.removed_rows, a.inserted_rows),
        (20, 0, 0)
    );
    // Still not `Exact`, and still warned about, however many paired: four rows had no key.
    assert_eq!(a.confidence, MatchConfidence::Medium);
    assert_eq!(
        warnings(s)[0].kind,
        DiagnosticKind::MissingAlignmentKey {
            old_count: 4,
            new_count: 4
        }
    );
}

/// The change is not lost among the paired rows: only the changed keyless row is a removal plus an
/// insertion, and the diff is that row's cells — not every keyless row's.
#[test]
fn only_the_changed_keyless_row_is_reported_when_the_others_pair() {
    let cols = 4u16;
    // Row index 9 (0-based) is keyless (9 % 5 == 4); change its column C.
    let d = diff(
        sheet_with_periodic_blank_keys(20, cols, None),
        sheet_with_periodic_blank_keys(20, cols, Some((9, 2))),
        row_key(vec![1]),
    );
    let s = only_sheet(&d);
    let a = s.alignment_summary.as_ref().unwrap();
    assert_eq!(
        (a.matched_rows, a.removed_rows, a.inserted_rows),
        (19, 1, 1)
    );
    // One keyless row on each side, `cols - 1` cells each (column A is the blank key).
    assert_eq!(s.cell_diffs.len(), 2 * (cols as usize - 1));
    assert!(a1s(s).iter().all(|x| x.ends_with("10")), "{:?}", a1s(s));
    let text: Vec<String> = s
        .cell_diffs
        .iter()
        .filter_map(|c| c.value.as_ref())
        .flat_map(|v| [format!("{:?}", v.old), format!("{:?}", v.new)])
        .collect();
    assert!(text.iter().any(|t| t.contains("r9c2-CHANGED")), "{text:?}");
}

/// Identical keyless rows pair k-th with k-th, in row order, so unequal counts leave exactly the
/// surplus: three identical spacer rows in old and two in new leave one removal.
#[test]
fn unequal_counts_of_identical_keyless_rows_leave_the_surplus() {
    let old = wb_strings(&[
        (0, 0, "id1"),
        (1, 1, "spacer"),
        (2, 1, "spacer"),
        (3, 0, "id2"),
        (4, 1, "spacer"),
    ]);
    let new = wb_strings(&[
        (0, 0, "id1"),
        (1, 1, "spacer"),
        (2, 0, "id2"),
        (3, 1, "spacer"),
    ]);
    let d = diff(old, new, row_key(vec![1]));
    let s = only_sheet(&d);
    let a = s.alignment_summary.as_ref().unwrap();
    assert_eq!((a.matched_rows, a.removed_rows, a.inserted_rows), (4, 1, 0));
    // The surplus is the *last* old spacer (row 5): pairing is in row order.
    assert_eq!(a1s(s), vec!["B5"]);
}

/// The same on the new side: with more identical rows in new than in old, the rows reported as
/// inserted are the *later* ones. Which identical row pairs with which is unobservable in the
/// comparison, but which rows are left over is not — it decides the addresses a caller sees.
#[test]
fn a_surplus_of_identical_keyless_rows_on_the_new_side_is_the_later_ones() {
    let old = wb_strings(&[(0, 0, "id1"), (1, 1, "spacer")]);
    let new = wb_strings(&[
        (0, 0, "id1"),
        (1, 1, "spacer"),
        (2, 1, "spacer"),
        (3, 1, "spacer"),
    ]);
    let d = diff(old, new, row_key(vec![1]));
    let s = only_sheet(&d);
    let a = s.alignment_summary.as_ref().unwrap();
    assert_eq!((a.matched_rows, a.removed_rows, a.inserted_rows), (2, 0, 2));
    assert_eq!(a1s(s), vec!["B3", "B4"]);
}

/// Pairing is by values in the same columns: the same text in another column, or a number where the
/// other side has the same text, is a different row.
#[test]
fn rows_with_the_same_text_in_different_columns_do_not_pair() {
    let d = diff(
        wb_strings(&[(0, 0, "id1"), (1, 1, "note")]),
        wb_strings(&[(0, 0, "id1"), (1, 2, "note")]),
        row_key(vec![1]),
    );
    let a = only_sheet(&d).alignment_summary.as_ref().unwrap().clone();
    assert_eq!((a.matched_rows, a.removed_rows, a.inserted_rows), (1, 1, 1));
}

#[test]
fn a_number_and_the_same_text_do_not_pair() {
    let sheet = |number: bool| {
        let mut wb = rust_xlsxwriter::Workbook::new();
        let ws = wb.add_worksheet();
        ws.write_string(0, 0, "id1").unwrap();
        if number {
            ws.write_number(1, 1, 1.0).unwrap();
        } else {
            ws.write_string(1, 1, "1").unwrap();
        }
        wb.save_to_buffer().unwrap()
    };
    let d = diff(sheet(true), sheet(false), row_key(vec![1]));
    let a = only_sheet(&d).alignment_summary.as_ref().unwrap().clone();
    assert_eq!((a.matched_rows, a.removed_rows, a.inserted_rows), (1, 1, 1));
}

/// A paired row is still compared cell by cell: pairing changes which rows are *matched*, and
/// cannot hide a difference. Rows with the same values and a different formula pair, and the
/// formula change is reported.
#[test]
fn a_paired_row_is_still_compared_so_a_formula_change_is_reported() {
    let with_formula = |f: &str| {
        let mut wb = rust_xlsxwriter::Workbook::new();
        let ws = wb.add_worksheet();
        ws.write_string(0, 0, "id1").unwrap();
        // Keyless row: no cell in column A. Same cached value 2, different formulas.
        ws.write_formula(1, 1, rust_xlsxwriter::Formula::new(f).set_result("2"))
            .unwrap();
        wb.save_to_buffer().unwrap()
    };
    let d = diff(with_formula("=1+1"), with_formula("=2*1"), row_key(vec![1]));
    let s = only_sheet(&d);
    let a = s.alignment_summary.as_ref().unwrap();
    assert_eq!((a.matched_rows, a.removed_rows, a.inserted_rows), (2, 0, 0));
    assert_eq!(a1s(s), vec!["B2"], "the formula change must be reported");
    assert!(s.cell_diffs[0].formula.is_some());
}

// ---------------------------------------------------------------------------
// Only RowKey has the rule
// ---------------------------------------------------------------------------

/// `RowSignature` builds its sequence from every cell, so it has no keyless row; and `Positional`
/// has no alignment. Neither raises the warning.
#[test]
fn the_other_two_modes_never_raise_it() {
    for mode in [
        AlignmentMode::Positional,
        AlignmentMode::RowSignature {
            sample_columns: None,
        },
    ] {
        let d = diff(
            sheet_with_blank_key_row("note-old"),
            sheet_with_blank_key_row("note-new"),
            opts(mode.clone()),
        );
        assert!(warnings(only_sheet(&d)).is_empty(), "{mode:?}");
    }
}
