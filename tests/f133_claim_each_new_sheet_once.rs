//! Two old sheets claiming the same new sheet made a workbook differ from itself (f133).
//!
//! `match_sheets`'s exact-name phase (`src/matcher.rs`) paired an old sheet with the *first* new
//! sheet of the same name, without checking whether that new sheet was already claimed by an
//! earlier old sheet. Two old sheets sharing a name both matched the same new sheet; the genuine
//! second new sheet of that name was never considered and was reported `Added`, with its (actually
//! identical, on self-comparison) content read as spurious changes. Found by
//! `fuzz_self_comparison`'s oracle (M9 unit 01 / f132) — not a crash, a silent wrong answer.
//!
//! `match_sheets` itself is `pub(crate)`, so the constructed symmetric/asymmetric duplicate-name
//! cases live as unit tests in `src/matcher.rs`, next to the phase they exercise. This file is the
//! public-API-level regression: the exact fixture the fuzzer found, through `compare_bytes`.

use sheets_diff::{SheetChange, compare_bytes};

/// `fuzz/corpus/fuzz_self_comparison/self_chart_sheet`'s workbook payload with 3 bytes flipped
/// inside the compressed ZIP data by libFuzzer's coverage-guided mutation — still opens, still
/// parses as 3 sheets (the unmutated fixture has 2, both `Unchanged` on self-comparison), but the
/// corruption blanks more than one sheet's name to `""`.
const FIXTURE: &str = "tests/fixtures/f133/self-compare-duplicate-names.bin";

#[test]
fn the_fixture_compared_with_itself_reports_zero_cell_diffs() {
    let bytes = std::fs::read(FIXTURE).unwrap_or_else(|e| panic!("{FIXTURE}: {e}"));
    let diff = compare_bytes(&bytes, &bytes).expect("the fixture must open");

    assert_eq!(
        diff.summary.cells_changed, 0,
        "a workbook compared with itself reported {} changed cell(s)",
        diff.summary.cells_changed
    );
    for s in &diff.sheets {
        assert!(
            s.cell_diffs.is_empty(),
            "sheet {:?}: {} cell diff(s) against itself",
            s.old_sheet
                .as_ref()
                .or(s.new_sheet.as_ref())
                .map(|r| &r.name),
            s.cell_diffs.len()
        );
        assert!(
            matches!(s.change, SheetChange::Unchanged),
            "sheet {:?}: change was {:?}, not Unchanged",
            s.old_sheet
                .as_ref()
                .or(s.new_sheet.as_ref())
                .map(|r| &r.name),
            s.change
        );
        assert!(
            !matches!(s.change, SheetChange::Added | SheetChange::Removed),
            "self-comparison must never add or remove a sheet"
        );
    }
}

/// The number of sheets is itself part of the regression: the defect's symptom was one real sheet
/// silently absorbed into a spurious `Added`, so the count moved from 2 to 3.
#[test]
fn the_fixture_has_the_sheet_count_it_should_after_self_comparison() {
    let bytes = std::fs::read(FIXTURE).unwrap();
    let diff = compare_bytes(&bytes, &bytes).unwrap();
    assert_eq!(
        diff.sheets.len(),
        2,
        "{:?}",
        diff.sheets.iter().map(|s| &s.change).collect::<Vec<_>>()
    );
}
