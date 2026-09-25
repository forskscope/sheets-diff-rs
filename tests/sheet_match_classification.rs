//! How sheets are paired — the *classification*, not the reason (M10 unit 01).
//!
//! M10 unit 01 changed what the engine **says** about a rename (`SheetMatchReason`),
//! and only that. These tests pin what the engine **does**: which sheets pair with
//! which, as which `SheetChange`, at which `MatchConfidence`. They deliberately never
//! name a `SheetMatchReason`, so this file compiles and passes unchanged against the
//! 2.6.0 source as well — that is how "matching behaviour is unchanged" is shown
//! rather than asserted (the review request runs it against a `git archive` of the
//! 2.6.0 tag).
//!
//! * `every_corpus_scenario_keeps_its_2_6_0_classification` — the table below was
//!   produced by the 2.6.0 binary's `--format json` over all 19 corpus scenarios.
//! * The synthetic cases reach the matcher paths the corpus does not: the corpus has
//!   exactly one rename, on equal indices, in the default mode.

mod support;
use support::wb_sheets;

use sheets_diff::options::SheetMatchingMode;
use sheets_diff::{
    DiffOptions, MatchConfidence, SheetChange, WorkbookDiff, compare_bytes,
    compare_bytes_with_options,
};

/// `SheetChange` and, for a rename, its confidence — everything but the reason.
fn classify(change: &SheetChange) -> String {
    let conf = |c: &MatchConfidence| format!("{c:?}");
    match change {
        SheetChange::Unchanged => "Unchanged".into(),
        SheetChange::Modified => "Modified".into(),
        SheetChange::Added => "Added".into(),
        SheetChange::Removed => "Removed".into(),
        SheetChange::Moved => "Moved".into(),
        SheetChange::Renamed { confidence, .. } => format!("Renamed({})", conf(confidence)),
        SheetChange::RenamedAndMoved { confidence, .. } => {
            format!("RenamedAndMoved({})", conf(confidence))
        }
        other => panic!("unclassified SheetChange in this test: {other:?}"),
    }
}

/// One line per sheet pair, sorted, as `old→new:Classification` (`-` for an absent side).
fn pairs(diff: &WorkbookDiff) -> Vec<String> {
    let name = |s: &Option<sheets_diff::SheetRef>| {
        s.as_ref()
            .map(|r| r.name.clone())
            .unwrap_or_else(|| "-".into())
    };
    let mut v: Vec<String> = diff
        .sheets
        .iter()
        .map(|s| {
            format!(
                "{}→{}:{}",
                name(&s.old_sheet),
                name(&s.new_sheet),
                classify(&s.change)
            )
        })
        .collect();
    v.sort();
    v
}

fn with_mode(mode: SheetMatchingMode) -> DiffOptions {
    DiffOptions::builder().sheet_matching(mode).build().unwrap()
}

// ---------------------------------------------------------------------------
// The corpus, against the 2.6.0 baseline
// ---------------------------------------------------------------------------

/// `(scenario, classification of each sheet in result order)`, generated from the 2.6.0
/// binary's `--format json` (not typed by hand).
/// 22 sheets over 19 scenarios; only `renamed_sheet` carries a rename.
const BASELINE_2_6_0: &[(&str, &[&str])] = &[
    ("alignment_header_column", &["Modified"]),
    ("alignment_row_signature", &["Modified"]),
    ("chart_sheet", &["Modified", "Unchanged"]),
    ("date_column", &["Modified"]),
    ("empty_cell_before_content", &["Modified"]),
    ("empty_sheet", &["Unchanged"]),
    ("error_values", &["Modified"]),
    ("formula", &["Modified"]),
    ("formula_at_first_cell", &["Modified"]),
    ("formula_shifted_origin", &["Modified"]),
    ("iso_datetime", &["Modified"]),
    ("non_ascii_text", &["Modified"]),
    ("renamed_sheet", &["Renamed(Medium)"]),
    ("row_insertion_cascade", &["Modified"]),
    ("row_shifted_origin", &["Modified"]),
    ("sheet_reordered", &["Moved", "Moved", "Modified"]),
    ("sparse_range", &["Modified"]),
    ("typed_values", &["Modified"]),
    ("wide_columns", &["Modified"]),
];

#[test]
fn every_corpus_scenario_keeps_its_2_6_0_classification() {
    let root = std::path::Path::new("tests/fixtures/generated");
    let mut seen = 0;
    for (scenario, expected) in BASELINE_2_6_0 {
        let dir = root.join(scenario);
        let old = std::fs::read(dir.join("old.xlsx")).unwrap();
        let new = std::fs::read(dir.join("new.xlsx")).unwrap();
        let diff = compare_bytes(&old, &new).unwrap();
        let got: Vec<String> = diff.sheets.iter().map(|s| classify(&s.change)).collect();
        assert_eq!(&got, expected, "scenario `{scenario}`");
        seen += 1;
    }
    // Every scenario directory is in the table: a new scenario must add a row.
    let on_disk = std::fs::read_dir(root)
        .unwrap()
        .filter(|e| e.as_ref().unwrap().path().is_dir())
        .count();
    assert_eq!(on_disk, seen, "a corpus scenario has no baseline row");
}

// ---------------------------------------------------------------------------
// The matcher paths the corpus does not reach
// ---------------------------------------------------------------------------

const CELL: &[(u32, u16, &str)] = &[(0, 0, "x")];

/// Default mode, one unmatched sheet each side, **equal** tab position.
#[test]
fn default_mode_rename_on_equal_index() {
    let old = wb_sheets(&[("A", CELL), ("Keep", CELL)]);
    let new = wb_sheets(&[("B", CELL), ("Keep", CELL)]);
    let diff = compare_bytes(&old, &new).unwrap();
    assert_eq!(pairs(&diff), ["A→B:Renamed(Medium)", "Keep→Keep:Unchanged"]);
}

/// Default mode, one unmatched sheet each side, **different** tab positions: paired
/// because they are the only ones left.
#[test]
fn default_mode_rename_by_elimination_on_unequal_index() {
    // old: A(0) B(1)   new: B(0) C(1).  B matches by name (and moved); A and C are
    // the only unmatched sheets, at positions 0 and 1.
    let old = wb_sheets(&[("A", CELL), ("B", CELL)]);
    let new = wb_sheets(&[("B", CELL), ("C", CELL)]);
    let diff = compare_bytes(&old, &new).unwrap();
    assert_eq!(pairs(&diff), ["A→C:RenamedAndMoved(Low)", "B→B:Moved"]);
}

/// Default mode, two unmatched on each side: no rename is guessed.
#[test]
fn default_mode_ambiguous_candidates_are_not_paired() {
    let old = wb_sheets(&[("A", CELL), ("B", CELL)]);
    let new = wb_sheets(&[("X", CELL), ("Y", CELL)]);
    let diff = compare_bytes(&old, &new).unwrap();
    assert_eq!(
        pairs(&diff),
        ["-→X:Added", "-→Y:Added", "A→-:Removed", "B→-:Removed"]
    );
}

/// `ExactNameThenIndex`: the two-and-two case that default mode refuses is paired on
/// tab position, at `Low` confidence.
#[test]
fn index_mode_pairs_on_equal_index() {
    let old = wb_sheets(&[("A", CELL), ("B", CELL)]);
    let new = wb_sheets(&[("X", CELL), ("Y", CELL)]);
    let diff =
        compare_bytes_with_options(&old, &new, with_mode(SheetMatchingMode::ExactNameThenIndex))
            .unwrap();
    assert_eq!(pairs(&diff), ["A→X:Renamed(Low)", "B→Y:Renamed(Low)"]);
}

/// `ExactNameOnly` never pairs differently named sheets.
#[test]
fn exact_name_only_never_renames() {
    let old = wb_sheets(&[("A", CELL)]);
    let new = wb_sheets(&[("B", CELL)]);
    let diff = compare_bytes_with_options(&old, &new, with_mode(SheetMatchingMode::ExactNameOnly))
        .unwrap();
    assert_eq!(pairs(&diff), ["-→B:Added", "A→-:Removed"]);
}
