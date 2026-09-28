//! Building the set of cells to compare is linear in the sheet, not quadratic (f131).
//!
//! Under any non-`Positional` alignment, `build_sheet_diff` turns the row mapping into the set of
//! cells to compare with three loops — matched, removed and inserted rows. Each used to read a row's
//! cells with `map.keys().filter(|(row, _)| row == r)`: **a scan of the whole cell map once per
//! row**, O(rows x cells). It was the larger part of an aligned comparison (30 s of 30.3 s on 4,900
//! rows x 24 columns) and nothing in `Limits` bounded it: `max_alignment_product` counts
//! `old_rows x new_rows`, so **one row against 40,000 (product 40,000, default bound 25,000,000)
//! took 37 s where `Positional` took 0.13 s**. Each loop now uses `map.range((r, 0)..=(r, u32::MAX))`.
//!
//! **The assertions are ratios to `Positional` on the same pair, in the same process**, never a
//! wall-clock figure. `Positional` is linear in the cells, so an aligned comparison that is linear
//! too stays a small multiple of it; a quadratic one does not. The bound is **5x**: observed
//! 1.0-1.3x with the fix, and **11.8x-21.8x with any single scan restored** (the matched-row case,
//! which restores one side of two, is the tightest at ~12x), so the bound sits about 4x above the
//! fix and 2.4x below the closest mutant.
//! Each of the three loops has its own shape, so restoring any one scan fails a named test.

mod support;
use support::wb_strings;

use std::time::Instant;

use rust_xlsxwriter::Workbook;
use sheets_diff::options::AlignmentMode;
use sheets_diff::{DiffOptions, WorkbookDiff, compare_bytes_with_options};

/// `max_alignment_product` is switched off: what is under test is the cost of *building the compared-cell
/// set*, and the bound is about a different cost (the LCS table). It would also, at the sizes the
/// matched-row test needs, make alignment fall back to `Positional` and measure nothing.
fn opts(mode: AlignmentMode) -> DiffOptions {
    DiffOptions::builder()
        .max_alignment_product(None)
        .alignment(mode)
        .build()
        .unwrap()
}

/// `rows` rows of `cols` columns: a unique id in column A, short text elsewhere.
fn sheet(rows: u32, cols: u16, changed: bool) -> Vec<u8> {
    let mut wb = Workbook::new();
    let ws = wb.add_worksheet();
    for r in 0..rows {
        ws.write_string(r, 0, format!("ID{r:06}")).unwrap();
        for c in 1..cols {
            let mut v = format!("r{r}c{c}");
            if changed && r == 0 && c == 1 {
                v.push('!');
            }
            ws.write_string(r, c, v).unwrap();
        }
    }
    wb.save_to_buffer().unwrap()
}

/// Best of three runs, in seconds, and the result of the last: the minimum is the run least disturbed
/// by whatever else the machine was doing.
fn timed(old: &[u8], new: &[u8], mode: &AlignmentMode) -> (f64, WorkbookDiff) {
    let mut best = f64::MAX;
    let mut last = None;
    for _ in 0..3 {
        let start = Instant::now();
        let d = compare_bytes_with_options(old, new, opts(mode.clone())).unwrap();
        best = best.min(start.elapsed().as_secs_f64());
        last = Some(d);
    }
    (best, last.unwrap())
}

const MODES: fn() -> [(&'static str, AlignmentMode); 2] = || {
    [
        ("RowKey", AlignmentMode::RowKey { columns: vec![1] }),
        (
            "RowSignature",
            AlignmentMode::RowSignature {
                sample_columns: None,
            },
        ),
    ]
};

/// The bound. See the module comment.
const MAX_RATIO: f64 = 5.0;

fn assert_linear(old: &[u8], new: &[u8], what: &str) {
    assert_linear_in(old, new, what, MODES());
}

fn assert_linear_in(
    old: &[u8],
    new: &[u8],
    what: &str,
    modes: impl IntoIterator<Item = (&'static str, AlignmentMode)>,
) {
    let (positional, base) = timed(old, new, &AlignmentMode::Positional);
    for (label, mode) in modes {
        let (aligned, d) = timed(old, new, &mode);
        let ratio = aligned / positional;
        eprintln!(
            "[{what}] {label}: {:.1} ms against Positional {:.1} ms = {ratio:.2}x",
            aligned * 1000.0,
            positional * 1000.0
        );
        assert!(
            ratio < MAX_RATIO,
            "{what}: {label} took {ratio:.1}x Positional on the same pair — the compared-cell set is \
             being built in more than linear time"
        );
        // The same pair, so the same differences: one row against many is all-inserted / all-removed
        // apart from the row they share, which is identical.
        assert_eq!(
            d.sheets[0].cell_diffs.len(),
            base.sheets[0].cell_diffs.len(),
            "{what}: {label}"
        );
    }
}

// ---------------------------------------------------------------------------
// One shape per loop
// ---------------------------------------------------------------------------

/// **Inserted-row loop.** Old: one row. New: 10,000. Every row but the shared one is inserted.
#[test]
fn one_row_against_a_tall_sheet_costs_what_positional_does() {
    assert_linear(&sheet(1, 3, false), &sheet(10_000, 3, false), "1 x 10,000");
}

/// **Removed-row loop.** The same pair the other way round.
#[test]
fn a_tall_sheet_against_one_row_costs_what_positional_does() {
    assert_linear(&sheet(10_000, 3, false), &sheet(1, 3, false), "10,000 x 1");
}

/// **Matched-row loop.** Every row matches, and the LCS table is empty: the key column (99) selects
/// no cell, so every row is *keyless*, and f130's pairing of identical keyless rows puts all 10,000 of
/// them in `matched` without an LCS. That isolates the matched loop, which used to scan a 30,000-cell
/// map per matched row — 300 million steps, about 25x `Positional`. (`RowSignature` has no keyless
/// path, so this is `RowKey` only; its matched loop is covered by the LCS-fed cases in the
/// cancellation tests and, above, the 1 x N shapes that match one row.)
#[test]
fn ten_thousand_matched_rows_cost_a_small_multiple_of_positional() {
    assert_linear_in(
        &sheet(10_000, 3, false),
        &sheet(10_000, 3, false),
        "10,000 matched keyless rows",
        [(
            "RowKey (no key column populated)",
            AlignmentMode::RowKey { columns: vec![99] },
        )],
    );
}

// ---------------------------------------------------------------------------
// The range's bounds: what a row's range reaches, at both ends of the column axis
// ---------------------------------------------------------------------------

/// `(r, 0)..=(r, u32::MAX)` must reach the first column (A) and the last (XFD, column 16,384) of a
/// row in each loop, and must not reach a neighbouring row. Matched: the new side carries a column
/// the old lacks and the old a column the new lacks (the union is kept). Removed and inserted rows
/// carry both ends too.
#[test]
fn a_rows_range_reaches_column_a_and_column_xfd_in_all_three_loops() {
    let xfd = 16_383u16; // 0-based
    // Old: row 1 (id `k`) has A, C and XFD; row 2 (id `gone`) has A and XFD.
    let old = wb_strings(&[
        (0, 0, "k"),
        (0, 2, "only-old"),
        (0, xfd, "old-edge"),
        (1, 0, "gone"),
        (1, xfd, "gone-edge"),
    ]);
    // New: row 1 (id `k`) has A, B and XFD; row 3 (id `fresh`) has A and XFD. Row 2 is empty.
    let new = wb_strings(&[
        (0, 0, "k"),
        (0, 1, "only-new"),
        (0, xfd, "new-edge"),
        (2, 0, "fresh"),
        (2, xfd, "fresh-edge"),
    ]);
    let d =
        compare_bytes_with_options(&old, &new, opts(AlignmentMode::RowKey { columns: vec![1] }))
            .unwrap();
    let a1s: Vec<String> = d.sheets[0]
        .cell_diffs
        .iter()
        .map(|c| c.address.a1.clone())
        .collect();
    assert_eq!(
        a1s,
        // matched row 1: B1 (new only), C1 (old only), XFD1 (changed); removed row 2: A2, XFD2;
        // inserted row 3: A3, XFD3. Row order, then column order.
        vec!["B1", "C1", "XFD1", "A2", "XFD2", "A3", "XFD3"],
        "{a1s:?}"
    );
}
