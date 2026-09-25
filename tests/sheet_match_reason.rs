//! `SheetMatchReason` says what the matcher actually did (M10 unit 01).
//!
//! Through 2.6.0 every rename carried `IndexAndContent`, which asserts that cell
//! content was compared. The matcher inspects no cell content anywhere. The two
//! variants that replace it name the two things that really happen: the sheets sat at
//! the same index, or they were the last two left. The *classification* — which sheets
//! pair, as what, at which confidence — is pinned separately in
//! `sheet_match_classification.rs`, which never names a reason.

mod support;
use support::wb_sheets;

use sheets_diff::options::SheetMatchingMode;
use sheets_diff::{
    DiffOptions, SheetChange, SheetMatchReason, WorkbookDiff, compare_bytes,
    compare_bytes_with_options,
};

const CELL: &[(u32, u16, &str)] = &[(0, 0, "x")];

fn reason_of_rename(diff: &WorkbookDiff, new_name: &str) -> SheetMatchReason {
    let sheet = diff
        .sheets
        .iter()
        .find(|s| s.new_sheet.as_ref().map(|r| r.name.as_str()) == Some(new_name))
        .unwrap_or_else(|| panic!("no sheet named `{new_name}` in the result"));
    match &sheet.change {
        SheetChange::Renamed { reason, .. } | SheetChange::RenamedAndMoved { reason, .. } => {
            reason.clone()
        }
        other => panic!("`{new_name}` is not a rename: {other:?}"),
    }
}

/// A rename matched on equal indices reports the index variant.
#[test]
fn a_rename_on_equal_indices_reports_same_index() {
    let old = wb_sheets(&[("A", CELL), ("Keep", CELL)]);
    let new = wb_sheets(&[("B", CELL), ("Keep", CELL)]);
    let diff = compare_bytes(&old, &new).unwrap();
    assert_eq!(reason_of_rename(&diff, "B"), SheetMatchReason::SameIndex);
}

/// A rename matched by elimination, with **unequal** indices, reports the elimination
/// variant. Before M10 unit 01 this case had no true value: it was reported as
/// `IndexAndContent` although no content was compared and no index matched.
#[test]
fn a_rename_by_elimination_reports_sole_remaining_pair() {
    // old: A(0) B(1)   new: B(0) C(1). B pairs by name; A and C are what is left.
    let old = wb_sheets(&[("A", CELL), ("B", CELL)]);
    let new = wb_sheets(&[("B", CELL), ("C", CELL)]);
    let diff = compare_bytes(&old, &new).unwrap();
    assert_eq!(
        reason_of_rename(&diff, "C"),
        SheetMatchReason::SoleRemainingPair
    );
}

/// The two causes are two facts about how far to trust the pairing; they must not
/// collapse into one value.
#[test]
fn the_two_causes_are_distinguishable() {
    let (a, b) = (wb_sheets(&[("A", CELL)]), wb_sheets(&[("B", CELL)]));
    let same = compare_bytes(&a, &b).unwrap();
    let (old, new) = (
        wb_sheets(&[("A", CELL), ("K", CELL)]),
        wb_sheets(&[("K", CELL), ("B", CELL)]),
    );
    let elim = compare_bytes(&old, &new).unwrap();
    assert_ne!(reason_of_rename(&same, "B"), reason_of_rename(&elim, "B"));
}

/// `ExactNameThenIndex` pairs on position, so its renames are `SameIndex`.
#[test]
fn index_mode_renames_report_same_index() {
    let old = wb_sheets(&[("A", CELL), ("B", CELL)]);
    let new = wb_sheets(&[("X", CELL), ("Y", CELL)]);
    let opts = DiffOptions::builder()
        .sheet_matching(SheetMatchingMode::ExactNameThenIndex)
        .build()
        .unwrap();
    let diff = compare_bytes_with_options(&old, &new, opts).unwrap();
    assert_eq!(reason_of_rename(&diff, "X"), SheetMatchReason::SameIndex);
    assert_eq!(reason_of_rename(&diff, "Y"), SheetMatchReason::SameIndex);
}

/// The serialised value is the variant's name. `--format json` shipped in 2.6.0, so
/// this string is a machine-readable surface.
#[cfg(feature = "serde")]
#[test]
fn the_serialised_reason_is_the_new_name() {
    let old = wb_sheets(&[("A", CELL)]);
    let new = wb_sheets(&[("B", CELL)]);
    let diff = compare_bytes(&old, &new).unwrap();
    let json = sheets_diff::output::json::to_json(&diff).unwrap();
    assert!(json.contains(r#""reason":"SameIndex""#), "{json}");
    assert!(!json.contains("IndexAndContent"), "{json}");
}
