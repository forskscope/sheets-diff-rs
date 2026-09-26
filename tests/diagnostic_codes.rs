//! The diagnostic codes are the stable programmatic surface, so every one of them must be
//! something the engine can actually send (M10 unit 02).
//!
//! Through 2.6.0 the code table listed eleven strings; four could never arrive
//! (`formula_cached_value_unverified`, `unsupported_cell_value`, `datetime_not_normalized`,
//! `limit_truncated_cells`). A caller doing exactly what the docs say — match on `code()` —
//! wrote arms that could never run. These tests are written as **positives** so they compile
//! against the 2.6.0 source as well (no removed name appears here): the set of codes
//! producible is *exactly* the table, and nothing in the corpus falls outside it.
//!
//! The second half are **documentation guards** for the public values that stay although
//! the engine never produces them (RFC-037 §3.4). They look like tautologies. They are not
//! to be deleted as such: each one pins the sentence in that value's doc comment, so that a
//! change which starts producing the value fails a test and forces the doc — and the
//! decision — to be revisited.

mod support;
use support::{wb_numbers, wb_sheets, wb_strings};

use std::collections::BTreeSet;

use rust_xlsxwriter::Workbook;
use sheets_diff::options::AlignmentMode;
use sheets_diff::{
    CellDisplay, CellError, CellValue, Diagnostic, DiffOptions, DiffStage, DisplaySource,
    ObjectCompareMode, SheetMatchingMode, SourceKind, WorkbookDiff, compare_bytes,
    compare_bytes_with_options, compare_paths, compare_readers,
};

/// Every code the table in `DiagnosticKind::code()` lists, and nothing else.
const TABLE: &[&str] = &[
    "formula_unavailable",
    "ambiguous_sheet_match",
    "unsupported_workbook_feature",
    "unsupported_workbook_metadata",
    "defined_name_scope_unknown",
    "alignment_bound_exceeded",
    "duplicate_alignment_key",
    "missing_alignment_key",
];

// ---------------------------------------------------------------------------
// Walking a result
// ---------------------------------------------------------------------------

fn diagnostics(d: &WorkbookDiff) -> Vec<&Diagnostic> {
    let mut v: Vec<&Diagnostic> = d.diagnostics.iter().collect();
    for s in &d.sheets {
        v.extend(s.diagnostics.iter());
        for c in &s.cell_diffs {
            v.extend(c.diagnostics.iter());
        }
    }
    v
}

fn codes(d: &WorkbookDiff) -> BTreeSet<&'static str> {
    diagnostics(d).iter().map(|x| x.kind.code()).collect()
}

fn values(d: &WorkbookDiff) -> Vec<&CellValue> {
    d.sheets
        .iter()
        .flat_map(|s| s.cell_diffs.iter())
        .filter_map(|c| c.value.as_ref())
        .flat_map(|v| [&v.old, &v.new])
        .collect()
}

// ---------------------------------------------------------------------------
// Fixtures: one comparison per code, plus the corpus
// ---------------------------------------------------------------------------

fn corpus() -> Vec<(String, WorkbookDiff)> {
    let root = std::path::Path::new("tests/fixtures/generated");
    let mut out = Vec::new();
    for e in std::fs::read_dir(root).unwrap() {
        let dir = e.unwrap().path();
        if !dir.is_dir() {
            continue;
        }
        let old = std::fs::read(dir.join("old.xlsx")).unwrap();
        let new = std::fs::read(dir.join("new.xlsx")).unwrap();
        let name = dir.file_name().unwrap().to_string_lossy().into_owned();
        out.push((name, compare_bytes(&old, &new).unwrap()));
    }
    assert_eq!(out.len(), 19, "the corpus has 19 scenarios");
    out
}

fn with_defined_name(name: &str, target: &str) -> Vec<u8> {
    let mut wb = Workbook::new();
    let ws = wb.add_worksheet();
    ws.set_name("Sheet1").unwrap();
    ws.write_string(0, 0, "x").unwrap();
    wb.define_name(name, target).unwrap();
    wb.save_to_buffer().unwrap()
}

fn with_second_sheet_hidden(hidden: bool) -> Vec<u8> {
    let mut wb = Workbook::new();
    wb.add_worksheet().set_name("Front").unwrap();
    let ws = wb.add_worksheet();
    ws.set_name("Back").unwrap();
    ws.set_hidden(hidden);
    wb.save_to_buffer().unwrap()
}

fn row_key_opts(product: Option<u64>) -> DiffOptions {
    DiffOptions::builder()
        .max_alignment_product(product)
        .alignment(AlignmentMode::RowKey { columns: vec![1] })
        .build()
        .unwrap()
}

/// One synthetic comparison per producible code.
fn synthetic() -> Vec<(&'static str, WorkbookDiff)> {
    let cell: &[(u32, u16, &str)] = &[(0, 0, "x")];
    let nums = |o: f64| wb_numbers(&[(0, 0, 1.0 + o), (1, 0, 2.0 + o)]);
    vec![
        (
            "formula_unavailable",
            compare_bytes(nums(0.0), nums(0.5)).unwrap(),
        ),
        (
            "ambiguous_sheet_match",
            compare_bytes(
                wb_sheets(&[("A", cell), ("B", cell)]),
                wb_sheets(&[("X", cell), ("Y", cell)]),
            )
            .unwrap(),
        ),
        ("unsupported_workbook_feature", {
            let d = std::path::Path::new("tests/fixtures/generated/chart_sheet");
            compare_bytes(
                std::fs::read(d.join("old.xlsx")).unwrap(),
                std::fs::read(d.join("new.xlsx")).unwrap(),
            )
            .unwrap()
        }),
        (
            "defined_name_scope_unknown + unsupported_workbook_metadata (names)",
            compare_bytes(
                with_defined_name("Total", "=Sheet1!$A$1"),
                with_defined_name("Total", "=Sheet1!$A$2"),
            )
            .unwrap(),
        ),
        (
            "unsupported_workbook_metadata (visibility)",
            compare_bytes(
                with_second_sheet_hidden(false),
                with_second_sheet_hidden(true),
            )
            .unwrap(),
        ),
        (
            "alignment_bound_exceeded",
            compare_bytes_with_options(
                wb_strings(&[(0, 0, "id1"), (1, 0, "id2"), (2, 0, "id3")]),
                wb_strings(&[(0, 0, "id1"), (1, 0, "id2"), (2, 0, "id9")]),
                row_key_opts(Some(5)),
            )
            .unwrap(),
        ),
        (
            "duplicate_alignment_key",
            compare_bytes_with_options(
                wb_strings(&[(0, 0, "dup"), (1, 0, "dup"), (2, 0, "unique")]),
                wb_strings(&[(0, 0, "dup"), (1, 0, "unique")]),
                row_key_opts(None),
            )
            .unwrap(),
        ),
        (
            "missing_alignment_key",
            // Row 2 has no cell in the key column (A): unmatched, not absent.
            compare_bytes_with_options(
                wb_strings(&[(0, 0, "id1"), (1, 1, "note"), (2, 0, "id3")]),
                wb_strings(&[(0, 0, "id1"), (1, 1, "note"), (2, 0, "id3")]),
                row_key_opts(None),
            )
            .unwrap(),
        ),
    ]
}

// ---------------------------------------------------------------------------
// The table is exactly what can be produced
// ---------------------------------------------------------------------------

#[test]
fn every_code_in_the_table_is_producible_and_nothing_else_is() {
    let mut produced: BTreeSet<&str> = BTreeSet::new();
    for (label, d) in synthetic() {
        let got = codes(&d);
        assert!(
            !got.is_empty(),
            "`{label}` produced no diagnostic at all: the fixture no longer exercises its code"
        );
        produced.extend(got);
    }
    for (_, d) in corpus() {
        produced.extend(codes(&d));
    }
    let table: BTreeSet<&str> = TABLE.iter().copied().collect();
    assert_eq!(
        produced, table,
        "the codes the engine can produce differ from the documented table"
    );
}

#[test]
fn no_corpus_scenario_produces_a_code_outside_the_table() {
    let table: BTreeSet<&str> = TABLE.iter().copied().collect();
    for (scenario, d) in corpus() {
        let extra: Vec<_> = codes(&d).difference(&table).copied().collect();
        assert!(extra.is_empty(), "`{scenario}` produced {extra:?}");
    }
}

// ---------------------------------------------------------------------------
// Documentation guards — the values that stay although nothing produces them
// ---------------------------------------------------------------------------

/// `DiffStage::Open`, `::Normalize`, `::Aggregate` name pipeline stages that exist but to
/// which no diagnostic is attributed. Documented as vocabulary, not as outcomes.
#[test]
fn documentation_guard_no_diagnostic_is_attributed_to_the_three_unused_stages() {
    let mut stages: BTreeSet<String> = BTreeSet::new();
    let mut all: Vec<(String, WorkbookDiff)> = corpus();
    all.extend(synthetic().into_iter().map(|(l, d)| (l.to_string(), d)));
    for (_, d) in &all {
        for x in diagnostics(d) {
            stages.insert(format!("{:?}", x.location.stage));
        }
    }
    for unused in [DiffStage::Open, DiffStage::Normalize, DiffStage::Aggregate] {
        assert!(
            !stages.contains(&format!("{unused:?}")),
            "{unused:?} is now produced: update its doc comment and RFC-037 §3.4's decision"
        );
    }
}

/// `CellValue::Integer`, `::Duration`, `::Unsupported` and `CellError::Other` cannot arrive
/// through any `.xlsx` this crate accepts (M4 unit 01; `Other` is a forward-compatible
/// catch-all for an Excel error string we do not recognise).
#[test]
fn documentation_guard_no_cell_value_is_one_of_the_four_unreachable_shapes() {
    let mut all: Vec<WorkbookDiff> = corpus().into_iter().map(|(_, d)| d).collect();
    all.extend(synthetic().into_iter().map(|(_, d)| d));
    for d in &all {
        for v in values(d) {
            assert!(
                !matches!(
                    v,
                    CellValue::Integer(_) | CellValue::Duration(_) | CellValue::Unsupported { .. }
                ),
                "an unreachable CellValue shape arrived: {v:?}"
            );
            assert!(
                !matches!(v, CellValue::Error(CellError::Other(_))),
                "CellError::Other arrived: {v:?}"
            );
        }
    }
}

/// `ObjectCompareMode::CompareAvailable` is selectable and behaves as `WarnIfPresent`.
#[test]
fn documentation_guard_compare_available_behaves_as_warn_if_present() {
    let dir = std::path::Path::new("tests/fixtures/generated/chart_sheet");
    let old = std::fs::read(dir.join("old.xlsx")).unwrap();
    let new = std::fs::read(dir.join("new.xlsx")).unwrap();
    let with = |m: ObjectCompareMode| {
        let opts = DiffOptions::builder().object_mode(m).build().unwrap();
        compare_bytes_with_options(&old, &new, opts).unwrap()
    };
    assert_eq!(
        with(ObjectCompareMode::WarnIfPresent),
        with(ObjectCompareMode::CompareAvailable)
    );
}

/// This crate builds a `CellDisplay` only through `from_value`, and that only ever sets
/// `SheetsDiffDefault`. `ReaderProvided` and `ApplicationProvided` are vocabulary for a
/// *caller* who builds a `CellDisplay` themselves — they are reachable, by the caller.
#[test]
fn documentation_guard_this_crate_only_produces_the_default_display_source() {
    let samples = [
        CellValue::Empty,
        CellValue::Text("t".into()),
        CellValue::Number(1.5),
        CellValue::Bool(true),
        CellValue::Error(CellError::Div0),
    ];
    for v in &samples {
        assert_eq!(
            CellDisplay::from_value(v).source,
            DisplaySource::SheetsDiffDefault
        );
    }
    // ... and a caller may supply either of the others.
    for s in [
        DisplaySource::ReaderProvided,
        DisplaySource::ApplicationProvided,
    ] {
        assert_eq!(CellDisplay::new("shown".into(), None, s).source, s);
    }
}

// ---------------------------------------------------------------------------
// SourceKind: one kind per entry point, nothing left over
// ---------------------------------------------------------------------------

#[test]
fn each_entry_point_reports_its_own_source_kind() {
    let a = wb_strings(&[(0, 0, "a")]);
    let b = wb_strings(&[(0, 0, "b")]);

    let by_bytes = compare_bytes(&a, &b).unwrap();
    assert_eq!(by_bytes.old.source.kind, SourceKind::Bytes);
    assert_eq!(by_bytes.new.source.kind, SourceKind::Bytes);

    let by_reader = compare_readers(
        std::io::Cursor::new(a.clone()),
        std::io::Cursor::new(b.clone()),
    )
    .unwrap();
    assert_eq!(by_reader.old.source.kind, SourceKind::Reader);

    let dir = std::path::Path::new("tests/fixtures/generated/date_column");
    let by_path = compare_paths(dir.join("old.xlsx"), dir.join("new.xlsx")).unwrap();
    assert_eq!(by_path.old.source.kind, SourceKind::Path);
    assert_eq!(by_path.new.source.kind, SourceKind::Path);
}

/// `SheetMatchingMode` is referenced so this file keeps compiling if the mode list moves;
/// the ambiguous-match fixture above relies on the default mode refusing a 2-and-2 pairing.
#[test]
fn the_ambiguous_fixture_relies_on_the_default_matching_mode() {
    assert_eq!(
        DiffOptions::default().matching.sheet_matching,
        SheetMatchingMode::ExactNameThenConservativeRename
    );
}
