//! `AlignmentMode::RowSignature { sample_columns: Some(cols) }` must not lose a row that has no
//! cell in any sampled column (the-row-that-vanishes/01).
//!
//! Through 3.3.0, `compute_row_signatures` put a row in its map only if one of its cells passed
//! the `sample_columns` filter. A row with no cell in any sampled column was in none of `matched`,
//! `removed` or `inserted`, so it was not compared at all, and the result said `confidence: Exact`
//! about a sheet it had not looked at. A change in such a row was reported as no change — f130's
//! defect (`tests/rowkey_keyless_rows.rs`), on the path f130 did not touch.
//!
//! **The rule now, shared with the `RowKey` rescue via `unmapped_rows`/`pair_identical_rows`:** a
//! row with no cell in any sampled column is never absent. Rows with the same values in the same
//! columns on both sides are paired with one another, in row order; the rest are reported as
//! removed (old side) or inserted (new side), so a changed unsampled row reaches the comparison as
//! a whole-row change. A `missing_row_signature` warning names the count on each side. Unlike the
//! `RowKey` rescue, confidence carries **no additional clamp** here: including the rescued rows in
//! the counts is enough on its own to move `confidence_for` off `Exact` wherever it was wrongly
//! `Exact` before. `sample_columns: None` samples every cell, so no row can ever be excluded and
//! this rescue is always a no-op there.

mod support;
use support::wb_strings;

use sheets_diff::options::AlignmentMode;
use sheets_diff::{
    CellChangeKind, DiagnosticKind, DiffOptions, MatchConfidence, Severity, SheetDiff,
    WorkbookDiff, compare_bytes_with_options,
};

const CODE: &str = "missing_row_signature";

/// Two rows. Row 1: `A="K"`, `B="a"` — sampled, unchanged. Row 2: **no cell in column A**, only
/// `B`, which changes. Column A (1-based col 1) is the sampled column — the handoff's own
/// reproduction.
fn sheet_with_unsampled_row(row2_b: &str) -> Vec<u8> {
    wb_strings(&[(0, 0, "K"), (0, 1, "a"), (1, 1, row2_b)])
}

fn opts(mode: AlignmentMode) -> DiffOptions {
    DiffOptions::builder().alignment(mode).build().unwrap()
}

fn row_signature(sample_columns: Option<Vec<u32>>) -> DiffOptions {
    opts(AlignmentMode::RowSignature { sample_columns })
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
// Test 1 — the defect, and its control (failing-first)
// ---------------------------------------------------------------------------

/// The handoff's reproduction: `before` -> `AFTER` in a row with no cell in the sampled column
/// must be reported, with the address and both values — not merely a non-zero count.
#[test]
fn a_change_in_an_unsampled_row_is_reported_under_rowsignature() {
    let d = diff(
        sheet_with_unsampled_row("before"),
        sheet_with_unsampled_row("AFTER"),
        row_signature(Some(vec![1])),
    );
    let s = only_sheet(&d);
    assert!(
        !s.cell_diffs.is_empty(),
        "the changed cell in the unsampled row was not compared (cell_diffs is empty; \
         alignment_summary = {:?})",
        s.alignment_summary
    );
    // The unmapped row is unmatched (removed on the old side, inserted on the new), so it reaches
    // the comparison as two entries at the same address -- the old value leaving, the new value
    // arriving -- not one Modified entry. (This is row-space-and-the-anchor's subject: nothing in
    // today's public API distinguishes them beyond this test reading both sides out by hand.)
    assert!(a1s(s).iter().all(|a| a == "B2"), "{:?}", a1s(s));
    let text: Vec<String> = s
        .cell_diffs
        .iter()
        .filter_map(|c| c.value.as_ref())
        .flat_map(|v| [format!("{:?}", v.old), format!("{:?}", v.new)])
        .collect();
    assert!(text.iter().any(|t| t.contains("before")), "{text:?}");
    assert!(text.iter().any(|t| t.contains("AFTER")), "{text:?}");
}

/// The control: the same workbooks under `Positional` report the same change, so the fixture is
/// sound and the difference above is the alignment's doing.
#[test]
fn the_same_workbooks_under_positional_report_the_change() {
    let d = diff(
        sheet_with_unsampled_row("before"),
        sheet_with_unsampled_row("AFTER"),
        opts(AlignmentMode::Positional),
    );
    assert_eq!(a1s(only_sheet(&d)), vec!["B2"]);
}

/// Acceptance criterion 3: `confidence` is no longer `Exact` on the reproduction, as a consequence
/// of the counts moving — not because confidence was touched directly.
#[test]
fn confidence_is_not_exact_on_the_reproduction() {
    let d = diff(
        sheet_with_unsampled_row("before"),
        sheet_with_unsampled_row("AFTER"),
        row_signature(Some(vec![1])),
    );
    let a = only_sheet(&d).alignment_summary.as_ref().unwrap();
    assert_ne!(a.confidence, MatchConfidence::Exact, "{a:?}");
}

// ---------------------------------------------------------------------------
// Test 2 — `sample_columns: None` is unchanged
// ---------------------------------------------------------------------------

#[test]
fn sample_columns_none_was_already_correct_and_stays_so() {
    let d = diff(
        sheet_with_unsampled_row("before"),
        sheet_with_unsampled_row("AFTER"),
        row_signature(None),
    );
    let s = only_sheet(&d);
    assert!(a1s(s).iter().all(|a| a == "B2"), "{:?}", a1s(s));
    assert!(
        warnings(s).is_empty(),
        "every cell is sampled with None, so no row can be unmapped: {:?}",
        s.diagnostics
    );
}

// ---------------------------------------------------------------------------
// The warning
// ---------------------------------------------------------------------------

#[test]
fn the_warning_fires_with_the_count_on_each_side() {
    // Old has two unsampled rows (3 and 5); new has one (3). Column A (col 1) is sampled.
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
    let d = diff(old, new, row_signature(Some(vec![1])));
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
        DiagnosticKind::MissingRowSignature {
            old_count: 2,
            new_count: 1
        }
    );
    assert_eq!(w[0].location.sheet_order, Some(0));
    assert_eq!(w[0].location.sheet_name.as_deref(), Some("Sheet1"));
    assert!(w[0].location.address.is_none());
    assert!(d.diagnostics.iter().all(|x| x.kind.code() != CODE));
    assert_eq!(d.summary.diagnostics.warnings, 1);
}

// ---------------------------------------------------------------------------
// Identical unsampled rows pair; a changed one does not
// ---------------------------------------------------------------------------

/// Test 4: an unchanged row with no sampled cell must not become a removal plus an insertion.
#[test]
fn a_workbook_compared_with_itself_is_unchanged_under_rowsignature_even_with_unsampled_rows() {
    let b = sheet_with_unsampled_row("same");
    let d = diff(b.clone(), b, row_signature(Some(vec![1])));
    let s = only_sheet(&d);
    assert!(s.cell_diffs.is_empty(), "{:?}", a1s(s));
    assert_eq!(format!("{:?}", s.change), "Unchanged");
    let a = s.alignment_summary.as_ref().unwrap();
    assert_eq!((a.matched_rows, a.removed_rows, a.inserted_rows), (2, 0, 0));
    assert_eq!(
        warnings(s)[0].kind,
        DiagnosticKind::MissingRowSignature {
            old_count: 1,
            new_count: 1
        }
    );
}

/// Only the changed unsampled row is reported when another, identical one pairs silently.
#[test]
fn only_the_changed_unsampled_row_is_reported_when_another_pairs() {
    let old = wb_strings(&[
        (0, 0, "K"),
        (0, 1, "a"),
        (1, 1, "spacer"), // unsampled, identical both sides -> pairs silently
        (2, 1, "before"), // unsampled, changes
    ]);
    let new = wb_strings(&[(0, 0, "K"), (0, 1, "a"), (1, 1, "spacer"), (2, 1, "AFTER")]);
    let d = diff(old, new, row_signature(Some(vec![1])));
    let s = only_sheet(&d);
    let a = s.alignment_summary.as_ref().unwrap();
    // Row1 sampled+matched; row2 ("spacer") pairs with itself; row3 ("before"/"AFTER") is the
    // surplus on both sides once the identical pair above has claimed the earlier rows.
    assert_eq!((a.matched_rows, a.removed_rows, a.inserted_rows), (2, 1, 1));
    assert!(a1s(s).iter().all(|a| a == "B3"), "{:?}", a1s(s));
}

// ---------------------------------------------------------------------------
// Test 5 — the degenerate case: no row has a sampled cell (failing-first)
// ---------------------------------------------------------------------------

/// `sample_columns` naming a column no row populates makes *every* row unsampled. Mirrors
/// `rowkey_keyless_rows.rs`'s `a_key_column_no_row_populates_compares_everything_and_warns`:
/// `DiffOptions` performs no validation that a named column exists on either side (confirmed by
/// reading `DiffOptions::validate`, which is a no-op), so this is a realistic input, not a
/// contrived one. Before this unit, `old_sigs`/`new_sigs` are both empty maps and `lcs_match`
/// reports `(0, 0, 0)` -- which `confidence_for` calls `Exact` -- over a comparison that looked at
/// nothing.
#[test]
fn a_sampled_column_no_row_populates_compares_everything_and_warns() {
    let d = diff(
        wb_strings(&[(0, 0, "a"), (1, 0, "b")]),
        wb_strings(&[(0, 0, "a"), (1, 0, "CHANGED")]),
        row_signature(Some(vec![9])),
    );
    let s = only_sheet(&d);
    assert!(!s.cell_diffs.is_empty(), "nothing was compared");
    assert_eq!(warnings(s).len(), 1);
    let a = s.alignment_summary.as_ref().unwrap();
    // Row 1 ("a") is identical on both sides and pairs with itself; row 2 changed and does not.
    assert_eq!((a.matched_rows, a.removed_rows, a.inserted_rows), (1, 1, 1));
    assert_ne!(a.confidence, MatchConfidence::Exact);
}

/// The same shape, phrased as the degenerate case itself: a workbook compared with itself, every
/// row unsampled, must still report nothing changed and must not panic, divide by zero, or claim
/// `Exact` over a comparison that looked at nothing.
#[test]
fn every_row_unsampled_compares_itself_as_unchanged_not_exact_over_nothing() {
    let b = wb_strings(&[(0, 0, "a"), (1, 0, "b"), (2, 0, "c")]);
    let d = diff(b.clone(), b, row_signature(Some(vec![9])));
    let s = only_sheet(&d);
    assert!(s.cell_diffs.is_empty(), "{:?}", a1s(s));
    let a = s.alignment_summary.as_ref().unwrap();
    assert_eq!((a.matched_rows, a.removed_rows, a.inserted_rows), (3, 0, 0));
    // Every row paired by content, none by signature: still not `Exact`.
    assert_ne!(a.confidence, MatchConfidence::Exact, "{a:?}");
    assert_eq!(
        warnings(s)[0].kind,
        DiagnosticKind::MissingRowSignature {
            old_count: 3,
            new_count: 3
        }
    );
}

// ---------------------------------------------------------------------------
// Only `Some(cols)` can raise it
// ---------------------------------------------------------------------------

/// `RowKey` and `Positional` never raise this warning, nor does `RowSignature` with `None` (the
/// mirror image of `rowkey_keyless_rows.rs`'s `the_other_two_modes_never_raise_it`).
#[test]
fn only_row_signature_with_some_columns_raises_it() {
    for mode in [
        AlignmentMode::Positional,
        AlignmentMode::RowKey { columns: vec![1] },
        AlignmentMode::RowSignature {
            sample_columns: None,
        },
    ] {
        let d = diff(
            sheet_with_unsampled_row("before"),
            sheet_with_unsampled_row("AFTER"),
            opts(mode.clone()),
        );
        assert!(warnings(only_sheet(&d)).is_empty(), "{mode:?}");
    }
}

// ---------------------------------------------------------------------------
// Test 4b — the consumer's forty-row sheet
// ---------------------------------------------------------------------------

/// Forty rows, sampled on columns A and B. Row 20 (1-based) is a spacer populated only in column
/// C, which is not sampled; its value changes. The two-row reproduction above can be dismissed as
/// degenerate. This one cannot: the summary a consumer believes says `Exact` over forty rows, and
/// the spacer's change must still be reported.
#[test]
fn a_forty_row_sheet_with_an_unsampled_spacer_reports_its_change_and_is_not_exact() {
    fn forty_rows(spacer: &str) -> Vec<u8> {
        let mut cells: Vec<(u32, u16, String)> = Vec::new();
        for r in 0..40u32 {
            if r == 19 {
                cells.push((r, 2, spacer.to_string()));
            } else {
                cells.push((r, 0, format!("id{r:02}")));
                cells.push((r, 1, format!("v{r:02}")));
            }
        }
        let refs: Vec<(u32, u16, &str)> =
            cells.iter().map(|(r, c, v)| (*r, *c, v.as_str())).collect();
        wb_strings(&refs)
    }
    let d = diff(
        forty_rows("note-before"),
        forty_rows("note-AFTER"),
        row_signature(Some(vec![1, 2])),
    );
    let s = only_sheet(&d);

    // The change is reported, at the spacer's address, as the old value leaving and the new arriving.
    let cells = &s.cell_diffs;
    assert_eq!(cells.len(), 2, "{cells:?}");
    assert!(cells.iter().all(|c| c.address.a1 == "C20"), "{cells:?}");
    let mut kinds: Vec<CellChangeKind> = cells.iter().map(|c| c.change_kind()).collect();
    kinds.sort_by_key(|k| format!("{k:?}"));
    assert_eq!(kinds, vec![CellChangeKind::Added, CellChangeKind::Removed]);

    // The summary: 39 sampled rows matched by signature; the spacer is one removal and one insertion.
    let a = s.alignment_summary.as_ref().unwrap();
    assert_eq!(
        (a.matched_rows, a.removed_rows, a.inserted_rows),
        (39, 1, 1),
        "{a:?}"
    );
    assert_ne!(a.confidence, MatchConfidence::Exact, "{a:?}");
}
