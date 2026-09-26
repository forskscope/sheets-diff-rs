//! `DiagnosticLocation` says where a diagnostic came from — or says, deliberately, that it is not about
//! a sheet at all (M10 unit 04).
//!
//! **The rule.** A diagnostic that concerns a particular sheet names it — `sheet_order` and `sheet_name`
//! together, never one without the other — as that sheet is in the workbook the diagnostic is about; for a
//! diagnostic about a matched *pair* of sheets that is the new workbook's, exactly as the text renderer
//! labels a sheet. A diagnostic that is not about a particular sheet leaves both `None`, which means
//! **"not about a sheet"**, not "nobody set this".
//!
//! These tests pin the rule in both directions: sheet-level diagnostics carry their sheet, and
//! workbook-level ones deliberately carry none. Before this unit the two alignment warnings — sheet-level
//! diagnostics pushed into a specific `SheetDiff` — recorded nothing, and the visibility diagnostic
//! carried a name without an order.

mod support;
use support::{wb_sheets, wb_strings};

use rust_xlsxwriter::Workbook;
use sheets_diff::options::AlignmentMode;
use sheets_diff::{
    Diagnostic, DiffOptions, SheetDiff, WorkbookDiff, compare_bytes, compare_bytes_with_options,
};

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

fn row_key(product: Option<u64>) -> DiffOptions {
    DiffOptions::builder()
        .max_alignment_product(product)
        .alignment(AlignmentMode::RowKey { columns: vec![1] })
        .build()
        .unwrap()
}

/// Two rows share the key `dup` on the old side: raises `duplicate_alignment_key`.
const DUP_OLD: &[(u32, u16, &str)] = &[(0, 0, "dup"), (1, 0, "dup"), (2, 0, "unique")];
const DUP_NEW: &[(u32, u16, &str)] = &[(0, 0, "dup"), (1, 0, "unique")];
/// Row 2 has no cell in the key column (A) and changed between the two: raises `missing_alignment_key`.
const MISSING_OLD: &[(u32, u16, &str)] = &[(0, 0, "id1"), (1, 1, "note-old")];
const MISSING_NEW: &[(u32, u16, &str)] = &[(0, 0, "id1"), (1, 1, "note-new")];
/// 3 x 3 distinct rows: a product of 9 exceeds a bound of 5, raising `alignment_bound_exceeded`.
const BOUND_OLD: &[(u32, u16, &str)] = &[(0, 0, "id1"), (1, 0, "id2"), (2, 0, "id3")];
const BOUND_NEW: &[(u32, u16, &str)] = &[(0, 0, "id1"), (1, 0, "id2"), (2, 0, "id9")];

fn with_defined_name(name: &str, target: &str) -> Vec<u8> {
    let mut wb = Workbook::new();
    let ws = wb.add_worksheet();
    ws.set_name("Sheet1").unwrap();
    ws.write_string(0, 0, "x").unwrap();
    wb.define_name(name, target).unwrap();
    wb.save_to_buffer().unwrap()
}

/// Sheets named in `order`, with the named ones hidden.
fn workbook_with_hidden(order: &[&str], hidden: &[&str]) -> Vec<u8> {
    let mut wb = Workbook::new();
    for name in order {
        let ws = wb.add_worksheet();
        ws.set_name(*name).unwrap();
        ws.set_hidden(hidden.contains(name));
    }
    wb.save_to_buffer().unwrap()
}

fn all_diagnostics(d: &WorkbookDiff) -> Vec<&Diagnostic> {
    let mut v: Vec<&Diagnostic> = d.diagnostics.iter().collect();
    for s in &d.sheets {
        v.extend(s.diagnostics.iter());
        for c in &s.cell_diffs {
            v.extend(c.diagnostics.iter());
        }
    }
    v
}

fn only<'a>(d: &'a WorkbookDiff, code: &str) -> &'a Diagnostic {
    let hits: Vec<_> = all_diagnostics(d)
        .into_iter()
        .filter(|x| x.kind.code() == code)
        .collect();
    assert_eq!(
        hits.len(),
        1,
        "expected exactly one `{code}`, got {}",
        hits.len()
    );
    hits[0]
}

/// The sheet the renderer labels an owning `SheetDiff` with: the new side, else the old one.
fn label(s: &SheetDiff) -> (usize, String) {
    let r = s.new_sheet.as_ref().or(s.old_sheet.as_ref()).unwrap();
    (r.index, r.name.clone())
}

fn location_pair(x: &Diagnostic) -> (Option<usize>, Option<String>) {
    (x.location.sheet_order, x.location.sheet_name.clone())
}

// ---------------------------------------------------------------------------
// The two sites that recorded nothing
// ---------------------------------------------------------------------------

#[test]
fn duplicate_alignment_key_names_its_sheet_by_name_and_order() {
    let old = wb_sheets(&[("Data", DUP_OLD)]);
    let new = wb_sheets(&[("Data", DUP_NEW)]);
    let d = compare_bytes_with_options(&old, &new, row_key(None)).unwrap();
    let x = only(&d, "duplicate_alignment_key");
    assert_eq!(location_pair(x), (Some(0), Some("Data".into())));
    // ... and it is the same sheet the owning SheetDiff is.
    let (order, name) = label(&d.sheets[0]);
    assert_eq!(location_pair(x), (Some(order), Some(name)));
    // it sits in that sheet's own vector, not the workbook's
    assert!(d.sheets[0].diagnostics.iter().any(|y| std::ptr::eq(y, x)));
}

#[test]
fn alignment_bound_exceeded_names_its_sheet_by_name_and_order() {
    let old = wb_sheets(&[("Front", &[]), ("Data", BOUND_OLD)]);
    let new = wb_sheets(&[("Front", &[]), ("Data", BOUND_NEW)]);
    let d = compare_bytes_with_options(&old, &new, row_key(Some(5))).unwrap();
    let x = only(&d, "alignment_bound_exceeded");
    // The second sheet: the order is 1, not a default 0.
    assert_eq!(location_pair(x), (Some(1), Some("Data".into())));
}

/// For a diagnostic about a matched *pair*, the location names the NEW workbook's sheet — the label the
/// renderer uses — not the old one, when a rename or a move separates them.
#[test]
fn a_pair_level_diagnostic_names_the_new_side_of_a_renamed_and_moved_sheet() {
    // old: Old(0) with duplicate keys, Stay(1).   new: Stay(0), New(1).
    // Stay matches by name (and moved); Old and New are what is left: paired by elimination,
    // RenamedAndMoved, indices 0 -> 1.
    let old = wb_sheets(&[("Old", DUP_OLD), ("Stay", &[])]);
    let new = wb_sheets(&[("Stay", &[]), ("New", DUP_NEW)]);
    let d = compare_bytes_with_options(&old, &new, row_key(None)).unwrap();
    let x = only(&d, "duplicate_alignment_key");
    assert_eq!(
        location_pair(x),
        (Some(1), Some("New".into())),
        "the pair's new side (index 1, `New`), not the old side (index 0, `Old`)"
    );
}

/// The same rule for the third alignment warning, `missing_alignment_key`: on a sheet that was renamed
/// **and** moved it names the new workbook's sheet. All three alignment warnings get their sheet from one
/// place (`build_sheet_diff` passes `new_sheet.or(old_sheet)` into `compute_row_mapping`), which is why this
/// is correct — and why, until this test, nothing said so for this one.
#[test]
fn missing_alignment_key_names_the_new_side_of_a_renamed_and_moved_sheet() {
    // old: Old(0) with a keyless row, Stay(1).   new: Stay(0), New(1): `Old` -> `New` is RenamedAndMoved, 0 -> 1.
    let old = wb_sheets(&[("Old", MISSING_OLD), ("Stay", &[])]);
    let new = wb_sheets(&[("Stay", &[]), ("New", MISSING_NEW)]);
    let d = compare_bytes_with_options(&old, &new, row_key(None)).unwrap();
    assert!(
        d.sheets
            .iter()
            .any(|s| format!("{:?}", s.change).starts_with("RenamedAndMoved")),
        "the fixture must be a renamed-and-moved pair"
    );
    let x = only(&d, "missing_alignment_key");
    assert_eq!(
        location_pair(x),
        (Some(1), Some("New".into())),
        "the pair's new side (index 1, `New`), not the old side (index 0, `Old`)"
    );
}

/// A sheet that exists on one side only is named by that side, for the third warning too.
#[test]
fn a_one_sided_sheet_with_a_missing_key_is_named_by_the_side_it_exists_on() {
    let added = compare_bytes_with_options(
        wb_sheets(&[("Keep", &[])]),
        wb_sheets(&[("Keep", &[]), ("Extra", MISSING_NEW)]),
        row_key(None),
    )
    .unwrap();
    assert_eq!(
        location_pair(only(&added, "missing_alignment_key")),
        (Some(1), Some("Extra".into()))
    );
    let removed = compare_bytes_with_options(
        wb_sheets(&[("Keep", &[]), ("Gone", MISSING_OLD)]),
        wb_sheets(&[("Keep", &[])]),
        row_key(None),
    )
    .unwrap();
    assert_eq!(
        location_pair(only(&removed, "missing_alignment_key")),
        (Some(1), Some("Gone".into()))
    );
}

/// A sheet that exists only on the new side is named by it; one that exists only on the old side is
/// named by the old one (the same fallback the renderer has).
#[test]
fn a_one_sided_sheet_is_named_by_the_side_it_exists_on() {
    // Added sheet with duplicate keys.
    let added = compare_bytes_with_options(
        wb_sheets(&[("Keep", &[])]),
        wb_sheets(&[("Keep", &[]), ("Extra", DUP_OLD)]),
        row_key(None),
    )
    .unwrap();
    assert_eq!(
        location_pair(only(&added, "duplicate_alignment_key")),
        (Some(1), Some("Extra".into()))
    );
    // Removed sheet with duplicate keys.
    let removed = compare_bytes_with_options(
        wb_sheets(&[("Keep", &[]), ("Gone", DUP_OLD)]),
        wb_sheets(&[("Keep", &[])]),
        row_key(None),
    )
    .unwrap();
    assert_eq!(
        location_pair(only(&removed, "duplicate_alignment_key")),
        (Some(1), Some("Gone".into()))
    );
}

// ---------------------------------------------------------------------------
// The inconsistent one: a name without an order
// ---------------------------------------------------------------------------

#[test]
fn a_sheet_visibility_diagnostic_sets_both_fields() {
    let old = workbook_with_hidden(&["Front", "Back"], &[]);
    let new = workbook_with_hidden(&["Front", "Back"], &["Back"]);
    let d = compare_bytes(&old, &new).unwrap();
    let x = all_diagnostics(&d)
        .into_iter()
        .find(|x| x.kind.code() == "unsupported_workbook_metadata")
        .expect("a visibility change is reported");
    assert_eq!(location_pair(x), (Some(1), Some("Back".into())));
}

/// When the sheet's position differs between the workbooks the new workbook's position is the one named.
#[test]
fn a_moved_sheets_visibility_diagnostic_names_its_new_position() {
    // `Back` is at position 1 in the old workbook and 2 in the new one. (The first sheet of a
    // workbook cannot be hidden, so it is never the hidden one in these fixtures.)
    let old = workbook_with_hidden(&["A", "Back", "C"], &[]);
    let new = workbook_with_hidden(&["C", "A", "Back"], &["Back"]);
    let d = compare_bytes(&old, &new).unwrap();
    let x = all_diagnostics(&d)
        .into_iter()
        .find(|x| x.kind.code() == "unsupported_workbook_metadata")
        .expect("a visibility change is reported");
    assert_eq!(location_pair(x), (Some(2), Some("Back".into())));
}

// ---------------------------------------------------------------------------
// The other direction: workbook-level diagnostics deliberately name no sheet
// ---------------------------------------------------------------------------

#[test]
fn a_workbook_level_diagnostic_deliberately_names_no_sheet() {
    // Defined names are workbook-scoped here (scope is unavailable — that is what
    // `defined_name_scope_unknown` says), so neither they nor that note is about a sheet.
    let d = compare_bytes(
        with_defined_name("Total", "=Sheet1!$A$1"),
        with_defined_name("Total", "=Sheet1!$A$2"),
    )
    .unwrap();
    for code in [
        "defined_name_scope_unknown",
        "unsupported_workbook_metadata",
    ] {
        let x = only(&d, code);
        assert_eq!(
            location_pair(x),
            (None, None),
            "`{code}` is not about a sheet"
        );
    }

    // An ambiguous rename concerns several candidate sheets (listed in `kind`), not one.
    let cell: &[(u32, u16, &str)] = &[(0, 0, "x")];
    let amb = compare_bytes(
        wb_sheets(&[("A", cell), ("B", cell)]),
        wb_sheets(&[("X", cell), ("Y", cell)]),
    )
    .unwrap();
    assert_eq!(
        location_pair(only(&amb, "ambiguous_sheet_match")),
        (None, None)
    );

    // The blanket coverage note names no sheet; the per-sheet chart-sheet warnings do.
    let dir = std::path::Path::new("tests/fixtures/generated/chart_sheet");
    let chart = compare_bytes(
        std::fs::read(dir.join("old.xlsx")).unwrap(),
        std::fs::read(dir.join("new.xlsx")).unwrap(),
    )
    .unwrap();
    let features: Vec<_> = all_diagnostics(&chart)
        .into_iter()
        .filter(|x| x.kind.code() == "unsupported_workbook_feature")
        .collect();
    assert!(
        features.iter().any(|x| location_pair(x) == (None, None)),
        "the blanket note"
    );
    assert!(
        features.iter().any(|x| x.location.sheet_name.is_some()),
        "a per-sheet warning"
    );
}

// ---------------------------------------------------------------------------
// The guard on the rule, not on any one site
// ---------------------------------------------------------------------------

fn corpus_and_synthetic() -> Vec<(String, WorkbookDiff)> {
    let mut out = Vec::new();
    let root = std::path::Path::new("tests/fixtures/generated");
    for e in std::fs::read_dir(root).unwrap() {
        let dir = e.unwrap().path();
        if !dir.is_dir() {
            continue;
        }
        let name = dir.file_name().unwrap().to_string_lossy().into_owned();
        let old = std::fs::read(dir.join("old.xlsx")).unwrap();
        let new = std::fs::read(dir.join("new.xlsx")).unwrap();
        out.push((name, compare_bytes(&old, &new).unwrap()));
    }
    out.push((
        "dup keys".into(),
        compare_bytes_with_options(
            wb_sheets(&[("Data", DUP_OLD)]),
            wb_sheets(&[("Data", DUP_NEW)]),
            row_key(None),
        )
        .unwrap(),
    ));
    out.push((
        "bound exceeded".into(),
        compare_bytes_with_options(
            wb_strings(BOUND_OLD),
            wb_strings(BOUND_NEW),
            row_key(Some(5)),
        )
        .unwrap(),
    ));
    out.push((
        "renamed and moved, dup keys".into(),
        compare_bytes_with_options(
            wb_sheets(&[("Old", DUP_OLD), ("Stay", &[])]),
            wb_sheets(&[("Stay", &[]), ("New", DUP_NEW)]),
            row_key(None),
        )
        .unwrap(),
    ));
    out.push((
        "renamed and moved, missing key".into(),
        compare_bytes_with_options(
            wb_sheets(&[("Old", MISSING_OLD), ("Stay", &[])]),
            wb_sheets(&[("Stay", &[]), ("New", MISSING_NEW)]),
            row_key(None),
        )
        .unwrap(),
    ));
    out.push((
        "hidden".into(),
        compare_bytes(
            workbook_with_hidden(&["A", "Back", "C"], &[]),
            workbook_with_hidden(&["C", "A", "Back"], &["Back"]),
        )
        .unwrap(),
    ));
    out
}

/// Never one of the pair without the other — anywhere, in any result.
#[test]
fn no_diagnostic_sets_a_sheet_name_without_a_sheet_order_or_the_reverse() {
    for (label, d) in corpus_and_synthetic() {
        for x in all_diagnostics(&d) {
            assert_eq!(
                x.location.sheet_order.is_some(),
                x.location.sheet_name.is_some(),
                "`{label}`: `{}` sets exactly one of sheet_order / sheet_name: {:?}",
                x.kind.code(),
                x.location
            );
        }
    }
}

/// Every diagnostic that sits inside a `SheetDiff` was raised about *that* sheet: its location is one of
/// the sheet's own sides, and — when it is about the pair — the new side.
#[test]
fn every_sheet_level_diagnostic_names_the_sheet_that_owns_it() {
    let mut checked = 0;
    for (scenario, d) in corpus_and_synthetic() {
        for s in &d.sheets {
            let sides: Vec<(usize, String)> = [&s.old_sheet, &s.new_sheet]
                .into_iter()
                .flatten()
                .map(|r| (r.index, r.name.clone()))
                .collect();
            for x in &s.diagnostics {
                let (order, name) = location_pair(x);
                let (order, name) = (
                    order.unwrap_or_else(|| {
                        panic!(
                            "`{scenario}`: sheet-level `{}` names no sheet",
                            x.kind.code()
                        )
                    }),
                    name.unwrap(),
                );
                assert!(
                    sides.contains(&(order, name.clone())),
                    "`{scenario}`: `{}` names ({order}, `{name}`), which is neither side of its owning sheet {sides:?}",
                    x.kind.code()
                );
                if matches!(
                    x.kind.code(),
                    "alignment_bound_exceeded"
                        | "duplicate_alignment_key"
                        | "missing_alignment_key"
                ) {
                    assert_eq!(
                        (order, name),
                        label(s),
                        "`{scenario}`: a pair-level diagnostic names the new side"
                    );
                }
                checked += 1;
            }
        }
    }
    assert!(checked > 0, "the cross-check looked at nothing");
}
