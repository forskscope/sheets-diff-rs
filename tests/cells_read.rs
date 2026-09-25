//! `DiffMetrics::cells_read` and `Limits::max_cells_read` count **populated cells** (M10 unit 05).
//!
//! Through 2.6.0 both were the **area of the bounding box** of each sheet's populated cells, summed over
//! sheets and sides. That number was the memory a *dense* range allocated; since the read was made to
//! stream (2.5.1), nothing spends it, so the metric was geometry under a name that says cells and the
//! bound guarded a quantity nobody pays for. Both now count the cells the reader retained: one accumulator,
//! one meaning.
//!
//! **The relation to the old figure, proved and then tested.** A bounding box contains every cell in it, so
//! for any sheet `populated <= area`. The new count is therefore never larger than the old one, and
//! `max_cells_read` rejects **a subset** of what it rejected in 2.6.0: some workbooks that were refused
//! are now accepted (a large box, few cells); **none that passed is newly refused.**
//!
//! Tests that pass on the 2.6.0 source as well as here (the dense control, "still rejected") are marked
//! *(also true on 2.6.0)* — they are what stops "the metric was zeroed" from passing everything else.

mod support;
use support::{patch_xlsx_xml, wb_large, wb_sheets, wb_strings};

use std::io::Cursor;

use calamine::{Reader, Xlsx};
use rust_xlsxwriter::Workbook;
use sheets_diff::options::AlignmentMode;
use sheets_diff::{
    DiffOptions, LimitKind, Limits, SheetsDiffError, WorkbookDiff, compare_bytes,
    compare_bytes_with_options,
};

// ---------------------------------------------------------------------------
// The corpus: an independent count, and the 2.6.0 figures beside it
// ---------------------------------------------------------------------------

/// `(scenario, cells_read under 2.6.0 = bounding-box area)`, produced by the 2.6.0 binary's
/// `--format json` — not typed by hand.
const AREA_2_6_0: &[(&str, u64)] = &[
    ("alignment_header_column", 18),
    ("alignment_row_signature", 22),
    ("chart_sheet", 8),
    ("date_column", 4),
    ("empty_cell_before_content", 2),
    ("empty_sheet", 0),
    ("error_values", 4),
    ("formula", 4),
    ("formula_at_first_cell", 4),
    ("formula_shifted_origin", 6),
    ("iso_datetime", 4),
    ("non_ascii_text", 4),
    ("renamed_sheet", 4),
    ("row_insertion_cascade", 82),
    ("row_shifted_origin", 2),
    ("sheet_reordered", 6),
    ("sparse_range", 5200),
    ("typed_values", 6),
    ("wide_columns", 2),
];

/// Populated cells in a workbook, counted by **calamine directly** — an oracle that shares no code with
/// this crate's reader: every worksheet's non-empty cells.
fn populated(bytes: &[u8]) -> u64 {
    let mut wb: Xlsx<_> = calamine::open_workbook_from_rs(Cursor::new(bytes)).unwrap();
    let names = wb.sheet_names();
    let mut n = 0u64;
    for name in names {
        if let Ok(range) = wb.worksheet_range(&name) {
            n += range
                .used_cells()
                .filter(|(_, _, d)| !matches!(d, calamine::Data::Empty))
                .count() as u64;
        }
    }
    n
}

fn fixture(scenario: &str) -> (Vec<u8>, Vec<u8>) {
    let d = std::path::Path::new("tests/fixtures/generated").join(scenario);
    (
        std::fs::read(d.join("old.xlsx")).unwrap(),
        std::fs::read(d.join("new.xlsx")).unwrap(),
    )
}

#[test]
fn every_corpus_scenario_reports_the_populated_cell_count() {
    let mut moved = Vec::new();
    for (scenario, area) in AREA_2_6_0 {
        let (old, new) = fixture(scenario);
        let expected = populated(&old) + populated(&new);
        let got = compare_bytes(&old, &new).unwrap().metrics.cells_read;
        assert_eq!(
            got, expected,
            "`{scenario}`: cells_read is not the populated-cell count"
        );
        // The relation that makes the change a loosening and never a tightening.
        assert!(
            got <= *area,
            "`{scenario}`: {got} populated cells > {area} box area"
        );
        if got != *area {
            moved.push((*scenario, *area, got));
        }
    }
    // Which scenarios differ is a fact about the fixtures, recorded so a change is noticed: only 3 of 19
    // (a chart sheet, a formula-shifted origin, and the sparse box). The other 16 have no empty position
    // inside their box, so area and count coincide and their figure did not move.
    let names: Vec<&str> = moved.iter().map(|m| m.0).collect();
    assert_eq!(
        names,
        ["chart_sheet", "formula_shifted_origin", "sparse_range"],
        "the set of scenarios whose cells_read differs from 2.6.0: {moved:?}"
    );
}

/// `sparse_range` — two populated cells, `A1` and `Z100`, a 100 x 26 box: **the numbers invert.**
#[test]
fn sparse_range_reports_four_populated_cells_not_5200_of_area() {
    let (old, new) = fixture("sparse_range");
    let d = compare_bytes(&old, &new).unwrap();
    assert_eq!(
        d.metrics.cells_read, 4,
        "2 populated cells x 2 sides (2.6.0: 5200 = 2 x 2,600 of area)"
    );
    assert_eq!(d.metrics.cells_compared, 2, "the two coordinates compared");
    assert!(d.metrics.cells_read >= d.metrics.cells_compared);
}

// ---------------------------------------------------------------------------
// The control: where area and count coincide, nothing moves (also true on 2.6.0)
// ---------------------------------------------------------------------------

/// *(also true on 2.6.0)* A dense 5 x 4 block on each side: box area = populated count = 20, so
/// `cells_read` is 40 under either definition. Without this, a mistake that zeroes the metric would
/// pass every other test in the file.
#[test]
fn a_dense_block_reports_the_same_number_as_2_6_0() {
    let (old, new) = (wb_large(5, 4, "o"), wb_large(5, 4, "n"));
    let d = compare_bytes(&old, &new).unwrap();
    assert_eq!(d.metrics.cells_read, 40);
    assert_eq!(populated(&old) + populated(&new), 40);
    // and the corpus scenarios that are dense agree with their 2.6.0 figure
    for scenario in [
        "date_column",
        "formula",
        "renamed_sheet",
        "wide_columns",
        "non_ascii_text",
    ] {
        let area = AREA_2_6_0.iter().find(|r| r.0 == scenario).unwrap().1;
        let (o, n) = fixture(scenario);
        assert_eq!(
            compare_bytes(&o, &n).unwrap().metrics.cells_read,
            area,
            "{scenario}"
        );
    }
}

/// A styled blank cell is a record with no value: it is streamed (and counted toward the cancellation
/// poll) but is not a populated cell, so it is not counted here. 400 styled blanks and two populated
/// cells a side report 4. (2.6.0 reported 800: the two populated cells sit at opposite corners of a
/// 20 x 20 box, and it counted the box.)
#[test]
fn styled_blank_cells_are_not_counted() {
    let build = |tag: &str| {
        let bold = rust_xlsxwriter::Format::new().set_bold();
        let mut wb = Workbook::new();
        let ws = wb.add_worksheet();
        for r in 0..20u32 {
            for c in 0..20u16 {
                ws.write_blank(r, c, &bold).unwrap();
            }
        }
        ws.write_string(0, 0, tag).unwrap();
        ws.write_string(19, 19, tag).unwrap();
        wb.save_to_buffer().unwrap()
    };
    let d = compare_bytes(build("o"), build("n")).unwrap();
    assert_eq!(d.metrics.cells_read, 4);
}

// ---------------------------------------------------------------------------
// The limit: the pair that changes, both directions
// ---------------------------------------------------------------------------

/// A1 plus one stray cell at row 200,000: two populated cells, a box of 200,001 x 2 positions.
fn stray(tag: &str) -> Vec<u8> {
    let mut wb = Workbook::new();
    let ws = wb.add_worksheet();
    ws.write_string(0, 0, "a").unwrap();
    ws.write_string(200_000, 1, tag).unwrap();
    wb.save_to_buffer().unwrap()
}

fn bounded(n: u64) -> DiffOptions {
    DiffOptions::builder()
        .limits(Limits {
            max_cells_read: Some(n),
            ..Limits::default()
        })
        .build()
        .unwrap()
}

fn cells_read_error(r: Result<WorkbookDiff, SheetsDiffError>) -> Option<u64> {
    match r {
        Err(SheetsDiffError::LimitExceeded {
            limit: LimitKind::CellsRead,
            observed,
        }) => Some(observed),
        Ok(_) => None,
        Err(e) => panic!("unexpected error {e:?}"),
    }
}

/// **Newly accepted.** A large box with few populated cells: under a bound of 1,000 the box area
/// (400,002 per side) would have been refused in 2.6.0; the four populated cells are accepted now.
#[test]
fn a_large_box_with_few_cells_is_now_accepted() {
    let (old, new) = (stray("x"), stray("y"));
    let d = compare_bytes_with_options(&old, &new, bounded(1_000))
        .expect("4 populated cells, bound 1,000");
    assert_eq!(d.metrics.cells_read, 4);
}

/// *(also true on 2.6.0)* **Still refused.** Many populated cells in a small box: 40 x 40 = 1,600 cells
/// on one side under a bound of 1,000. Refused before and after — because a box always contains its cells,
/// so whatever exceeds the bound by count exceeded it by area first. This is why there is no
/// "newly refused" direction to test.
#[test]
fn many_cells_in_a_small_box_are_still_refused() {
    let (old, new) = (wb_large(40, 40, "o"), wb_large(40, 40, "n"));
    assert!(cells_read_error(compare_bytes_with_options(&old, &new, bounded(1_000))).is_some());
}

/// The bound fires on the *count*: at the cell that would make it `max + 1`.
#[test]
fn the_bound_reports_the_running_count_at_the_cell_that_broke_it() {
    let (old, new) = (wb_large(40, 40, "o"), wb_large(40, 40, "n"));
    assert_eq!(
        cells_read_error(compare_bytes_with_options(&old, &new, bounded(1_000))),
        Some(1_001)
    );
}

/// The metric and the bound are one number: a bound equal to `cells_read` passes, one less fails.
#[test]
fn a_bound_equal_to_the_metric_passes_and_one_less_fails() {
    let (old, new) = fixture("row_insertion_cascade");
    let n = compare_bytes(&old, &new).unwrap().metrics.cells_read;
    assert!(n > 1);
    assert!(compare_bytes_with_options(&old, &new, bounded(n)).is_ok());
    assert_eq!(
        cells_read_error(compare_bytes_with_options(&old, &new, bounded(n - 1))),
        Some(n)
    );
}

/// Cumulative across sheets and both sides, not per sheet.
#[test]
fn the_count_is_cumulative_across_sheets_and_sides() {
    let cells: &[(u32, u16, &str)] = &[(0, 0, "a"), (1, 0, "b"), (2, 0, "c")];
    let (old, new) = (
        wb_sheets(&[("A", cells), ("B", cells)]),
        wb_sheets(&[("A", cells), ("B", cells)]),
    );
    let d = compare_bytes(&old, &new).unwrap();
    assert_eq!(d.metrics.cells_read, 12, "2 sheets x 3 cells x 2 sides");
    // no single sheet reaches 5, the running total does
    assert_eq!(
        cells_read_error(compare_bytes_with_options(&old, &new, bounded(11))),
        Some(12)
    );
}

/// A repeated cell address replaces the cell rather than adding one, so the count is the map size — the
/// memory actually held — not the number of records streamed.
#[test]
fn a_repeated_address_is_counted_once() {
    let mut wb = Workbook::new();
    let ws = wb.add_worksheet();
    ws.write_number(0, 0, 1.0).unwrap();
    ws.write_number(0, 1, 5.0).unwrap();
    let base = wb.save_to_buffer().unwrap();
    // Duplicate the first record: <c r="A1"><v>1</v></c> appears twice.
    let dup = patch_xlsx_xml(&base, "xl/worksheets/sheet1.xml", |xml| {
        let needle = "<c r=\"A1\"><v>1</v></c>";
        assert!(xml.contains(needle), "unexpected sheet xml: {xml}");
        xml.replacen(needle, &format!("{needle}{needle}"), 1)
    });
    let d = compare_bytes(&dup, &base).unwrap();
    assert_eq!(
        d.metrics.cells_read, 4,
        "2 distinct addresses per side, however many records"
    );
}

// ---------------------------------------------------------------------------
// The invariant the doc claims
// ---------------------------------------------------------------------------

/// `cells_read >= cells_compared`: every coordinate compared came from at least one retained cell, and
/// each retained cell belongs to at most one coordinate. Checked over the corpus and over synthetic cases
/// that reach one-sided sheets and every alignment mode.
#[test]
fn cells_read_is_never_less_than_cells_compared() {
    let mut cases: Vec<(String, WorkbookDiff)> = Vec::new();
    for (scenario, _) in AREA_2_6_0 {
        let (o, n) = fixture(scenario);
        cases.push((scenario.to_string(), compare_bytes(&o, &n).unwrap()));
    }
    let a: &[(u32, u16, &str)] = &[
        (0, 0, "k1"),
        (0, 1, "a"),
        (1, 0, "k2"),
        (1, 1, "b"),
        (2, 0, "k3"),
        (2, 1, "c"),
    ];
    let b: &[(u32, u16, &str)] = &[
        (0, 0, "k0"),
        (0, 1, "z"),
        (1, 0, "k1"),
        (1, 1, "a"),
        (2, 0, "k3"),
        (2, 1, "q"),
    ];
    for (label, mode) in [
        ("row key", AlignmentMode::RowKey { columns: vec![1] }),
        (
            "row signature",
            AlignmentMode::RowSignature {
                sample_columns: None,
            },
        ),
        ("header column", AlignmentMode::HeaderColumn),
    ] {
        let opts = DiffOptions::builder().alignment(mode).build().unwrap();
        cases.push((
            format!("aligned: {label}"),
            compare_bytes_with_options(wb_strings(a), wb_strings(b), opts).unwrap(),
        ));
    }
    cases.push((
        "added and removed sheets".into(),
        compare_bytes(
            wb_sheets(&[("Keep", a), ("Gone", b)]),
            wb_sheets(&[("Keep", a), ("New", b)]),
        )
        .unwrap(),
    ));
    for (label, d) in &cases {
        assert!(
            d.metrics.cells_read >= d.metrics.cells_compared,
            "`{label}`: cells_read {} < cells_compared {}",
            d.metrics.cells_read,
            d.metrics.cells_compared
        );
        assert!(
            d.metrics.cells_compared >= d.metrics.diffs_emitted,
            "`{label}`"
        );
    }
}
