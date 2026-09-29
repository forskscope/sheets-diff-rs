//! A leftover sheet whose name collided with an unrelated matched pair's name vanished from the
//! result under `SheetMatchingMode::ExactNameThenIndex` (f134).
//!
//! Found while validating f133: that fix correctly stopped one old sheet from double-claiming
//! another's match, which let a genuinely-unmatched leftover sheet reach `ExactNameThenIndex`'s
//! "still unmatched" filters for the first time — and those filters checked whether *any*
//! already-matched pair shared the leftover's **name**, not whether it *was* the leftover, so a
//! name collision hid it. `WorkbookDiff::sheets` came out one entry short, silently: not
//! `Removed`, not `Added`, absent, with no diagnostic for a caller to notice. Fixed by comparing
//! `SheetRef::index` (unique per side) instead of `name` in both filters.
//!
//! `old-duplicate-name.xlsx` has two sheets both named `"Sheet1"` — `rust_xlsxwriter` refuses to
//! write that, so it was written with distinct names and then hand-edited (not fuzzer-mutated:
//! this is a deliberate, minimal XML edit, changing exactly one `<sheet name="...">` attribute in
//! `xl/workbook.xml`) to collide. `new-single-sheet.xlsx` has one sheet, also `"Sheet1"`.
//! `match_sheets`'s own unit tests (`src/matcher.rs`) cover the constructed shapes exhaustively;
//! this file is the public-API-level regression, through `compare_bytes_with_options` with
//! `ExactNameThenIndex` explicitly selected (not the default mode).

use sheets_diff::{DiffOptionsBuilder, SheetChange, SheetMatchingMode, compare_bytes_with_options};

const OLD: &str = "tests/fixtures/f134/old-duplicate-name.xlsx";
const NEW: &str = "tests/fixtures/f134/new-single-sheet.xlsx";

#[test]
fn the_leftover_sheet_is_removed_not_vanished() {
    let old = std::fs::read(OLD).unwrap_or_else(|e| panic!("{OLD}: {e}"));
    let new = std::fs::read(NEW).unwrap_or_else(|e| panic!("{NEW}: {e}"));
    let opts = DiffOptionsBuilder::new()
        .sheet_matching(SheetMatchingMode::ExactNameThenIndex)
        .build()
        .expect("valid options");

    let diff = compare_bytes_with_options(&old, &new, opts).expect("both fixtures must open");

    assert_eq!(
        diff.sheets.len(),
        2,
        "expected one matched pair and one Removed, got {:?} — a sheet vanished if this is 1",
        diff.sheets.iter().map(|s| &s.change).collect::<Vec<_>>()
    );

    let matched = diff
        .sheets
        .iter()
        .filter(|s| matches!(s.change, SheetChange::Unchanged | SheetChange::Modified))
        .count();
    let removed = diff
        .sheets
        .iter()
        .filter(|s| matches!(s.change, SheetChange::Removed))
        .count();
    assert_eq!(
        matched,
        1,
        "{:?}",
        diff.sheets.iter().map(|s| &s.change).collect::<Vec<_>>()
    );
    assert_eq!(
        removed,
        1,
        "{:?}",
        diff.sheets.iter().map(|s| &s.change).collect::<Vec<_>>()
    );
}
