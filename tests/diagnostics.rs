//! Diagnostic collection and reporting (M8 unit 02).
//!
//! Two separate things are pinned here, because they are two separate layers:
//!
//! * `DiagnosticOptions::min_severity` is a **collection** filter. A diagnostic
//!   below it is never kept, so it is absent from `WorkbookDiff::diagnostics` and
//!   from every `SheetDiff::diagnostics`, and the counters follow what was kept.
//!   No renderer is involved in any of those tests.
//! * The text renderers **display** diagnostics. `render_unified` shows warnings
//!   and errors, workbook-level and sheet-level; `render_summary` prints counts.
//!
//! (O3: before this unit, diagnostics attached to a *sheet* reached no output at
//! all — `DiffSummary` and both renderers looked only at the workbook-level
//! vector — including the two warnings whose whole job is to say "the diff you
//! are reading may be wrong".)

mod support;
use support::{wb_numbers, wb_strings};

use sheets_diff::options::{AlignmentMode, MatchingOptions};
use sheets_diff::output::text::{render_summary, render_unified};
use sheets_diff::{
    DiffOptions, Severity, SheetMatchingMode, WorkbookDiff, compare_bytes,
    compare_bytes_with_options,
};

// ---------------------------------------------------------------------------
// Fixtures, built in-test
// ---------------------------------------------------------------------------

/// `RowKey` alignment on column 1 — the mode that can raise sheet-level warnings.
fn row_key_options(
    min_severity: Option<Severity>,
    max_alignment_product: Option<u64>,
) -> DiffOptions {
    let mut opts = DiffOptions::builder()
        .max_alignment_product(max_alignment_product)
        .build_with_matching(MatchingOptions {
            sheet_matching: SheetMatchingMode::default(),
            alignment: AlignmentMode::RowKey { columns: vec![1] },
        })
        .unwrap();
    opts.diagnostics.min_severity = min_severity;
    opts
}

/// Two rows share the key `dup`: raises the sheet-level warning
/// `duplicate_alignment_key` under `RowKey` alignment.
fn duplicate_key_pair() -> (Vec<u8>, Vec<u8>) {
    (
        wb_strings(&[(0, 0, "dup"), (1, 0, "dup"), (2, 0, "unique")]),
        wb_strings(&[(0, 0, "dup"), (1, 0, "unique")]),
    )
}

/// 3 x 3 distinct rows = product 9, so a bound of 5 raises the sheet-level warning
/// `alignment_bound_exceeded` and the sheet is compared positionally instead.
fn bound_exceeded_pair() -> (Vec<u8>, Vec<u8>) {
    (
        wb_strings(&[(0, 0, "id1"), (1, 0, "id2"), (2, 0, "id3")]),
        wb_strings(&[(0, 0, "id1"), (1, 0, "id2"), (2, 0, "id9")]),
    )
}

/// Numeric cells with no formula: every one raises a sheet-level `Info`
/// `formula_unavailable`, plus the workbook-level `Info` coverage note.
const NUMERIC_CELLS: usize = 40;
fn numeric_pair() -> (Vec<u8>, Vec<u8>) {
    let cells = |offset: f64| -> Vec<(u32, u16, f64)> {
        (0..NUMERIC_CELLS as u32)
            .map(|r| (r, 0u16, r as f64 + offset))
            .collect()
    };
    (wb_numbers(&cells(0.0)), wb_numbers(&cells(0.5)))
}

// ---------------------------------------------------------------------------
// Helpers over a result
// ---------------------------------------------------------------------------

fn all_diagnostics(d: &WorkbookDiff) -> impl Iterator<Item = &sheets_diff::Diagnostic> {
    d.diagnostics
        .iter()
        .chain(d.sheets.iter().flat_map(|s| s.diagnostics.iter()))
}

fn count(d: &WorkbookDiff, severity: Severity) -> usize {
    all_diagnostics(d)
        .filter(|x| x.severity == severity)
        .count()
}

/// The summary's diagnostic total, all severities.
fn summary_total(d: &WorkbookDiff) -> usize {
    let s = &d.summary.diagnostics;
    s.errors + s.warnings + s.info
}

// ===========================================================================
// min_severity — a collection filter, observed with no renderer involved
// ===========================================================================

#[test]
fn min_severity_none_collects_every_diagnostic() {
    // The control: proves the filter is not always on.
    let (old, new) = numeric_pair();
    let d = compare_bytes(&old, &new).unwrap();

    assert!(
        d.diagnostics.iter().any(|x| x.severity == Severity::Info),
        "workbook-level Info"
    );
    assert!(
        d.sheets[0]
            .diagnostics
            .iter()
            .any(|x| x.severity == Severity::Info),
        "sheet-level Info"
    );
    assert!(d.summary.diagnostics.info > 0);
}

#[test]
fn min_severity_warning_drops_info_at_both_levels_and_the_counters_follow() {
    let (old, new) = numeric_pair();
    let mut opts = DiffOptions::default();
    opts.diagnostics.min_severity = Some(Severity::Warning);
    let d = compare_bytes_with_options(&old, &new, opts).unwrap();

    assert!(
        d.diagnostics.is_empty(),
        "workbook-level Info must be dropped: {:?}",
        d.diagnostics
    );
    assert!(
        d.sheets.iter().all(|s| s.diagnostics.is_empty()),
        "sheet-level Info must be dropped too, or the filter reproduces O3: {:?}",
        d.sheets.iter().map(|s| &s.diagnostics).collect::<Vec<_>>()
    );
    assert_eq!(
        d.summary.diagnostics.info, 0,
        "the counters follow what was kept"
    );
    assert_eq!(d.metrics.diagnostics_emitted, 0, "so does the metric");
}

#[test]
fn min_severity_error_drops_warnings_too() {
    let (old, new) = duplicate_key_pair();

    // Warning keeps the warning (>=) ...
    let kept =
        compare_bytes_with_options(&old, &new, row_key_options(Some(Severity::Warning), None))
            .unwrap();
    assert_eq!(
        count(&kept, Severity::Warning),
        1,
        "{:?}",
        kept.sheets[0].diagnostics
    );

    // ... Error drops it.
    let dropped =
        compare_bytes_with_options(&old, &new, row_key_options(Some(Severity::Error), None))
            .unwrap();
    assert_eq!(count(&dropped, Severity::Warning), 0);
    assert!(dropped.sheets[0].diagnostics.is_empty());
    assert_eq!(dropped.summary.diagnostics.warnings, 0);
}

// ===========================================================================
// O3 — a sheet's own diagnostics are counted, and shown
// ===========================================================================

#[test]
fn summary_counts_sheet_level_warnings() {
    let (old, new) = duplicate_key_pair();
    let d = compare_bytes_with_options(&old, &new, row_key_options(None, None)).unwrap();

    assert_eq!(
        d.sheets[0]
            .diagnostics
            .iter()
            .filter(|x| x.kind.code() == "duplicate_alignment_key")
            .count(),
        1,
        "precondition: the sheet carries the warning"
    );
    assert_eq!(
        d.summary.diagnostics.warnings,
        count(&d, Severity::Warning),
        "DiffSummary must count the sheet-level warning, not only the workbook-level vector"
    );
    assert_eq!(d.summary.diagnostics.warnings, 1);
}

#[test]
fn summary_total_equals_diagnostics_emitted_when_both_levels_have_diagnostics() {
    // Numeric cells: workbook-level Info (coverage note) + sheet-level Info per cell.
    let (old, new) = numeric_pair();
    let d = compare_bytes(&old, &new).unwrap();
    assert!(
        !d.diagnostics.is_empty() && !d.sheets[0].diagnostics.is_empty(),
        "precondition"
    );
    assert_eq!(summary_total(&d) as u64, d.metrics.diagnostics_emitted);

    // And with a filter applied, they still agree with each other and with the vectors.
    let (old, new) = duplicate_key_pair();
    for min in [
        None,
        Some(Severity::Info),
        Some(Severity::Warning),
        Some(Severity::Error),
    ] {
        let d = compare_bytes_with_options(&old, &new, row_key_options(min, None)).unwrap();
        assert_eq!(
            summary_total(&d),
            all_diagnostics(&d).count(),
            "min={min:?}"
        );
        assert_eq!(
            summary_total(&d) as u64,
            d.metrics.diagnostics_emitted,
            "min={min:?}"
        );
    }
}

#[test]
fn render_unified_shows_a_sheet_level_warning_and_names_the_sheet() {
    let (old, new) = duplicate_key_pair();
    let d = compare_bytes_with_options(&old, &new, row_key_options(None, None)).unwrap();
    let u = render_unified(&d);

    assert!(u.contains("# Diagnostics"), "no diagnostics section: {u}");
    assert!(
        u.contains("duplicate_alignment_key"),
        "the warning is not shown: {u}"
    );
    // The alignment diagnostics carry no `sheet_name` in their own location, so the
    // sheet is named from the SheetDiff that owns them.
    assert!(
        u.contains("sheet 'Sheet1'"),
        "the owning sheet is not named: {u}"
    );
}

#[test]
fn render_unified_shows_alignment_bound_exceeded() {
    // The sheet silently fell back to positional comparison. Nothing said so.
    let (old, new) = bound_exceeded_pair();
    let d = compare_bytes_with_options(&old, &new, row_key_options(None, Some(5))).unwrap();
    let u = render_unified(&d);
    assert!(u.contains("alignment_bound_exceeded"), "got: {u}");
}

#[test]
fn render_summary_counts_a_sheet_level_warning() {
    let (old, new) = duplicate_key_pair();
    let d = compare_bytes_with_options(&old, &new, row_key_options(None, None)).unwrap();
    let s = render_summary(&d);
    assert!(
        s.contains("diagnostics: 0 error(s), 1 warning(s)"),
        "got: {s}"
    );
}

#[test]
fn render_unified_leaves_workbook_level_warnings_where_they_were() {
    // A change to the diagnostics section must not move what it already showed.
    // Sheet-level entries come after workbook-level ones.
    let (old, new) = duplicate_key_pair();
    let d = compare_bytes_with_options(&old, &new, row_key_options(None, None)).unwrap();
    let u = render_unified(&d);
    let section = u.split("# Diagnostics").nth(1).unwrap();
    // Every printed entry is at Warning or above: the display threshold is unchanged.
    for line in section.lines().filter(|l| l.trim_start().starts_with('[')) {
        assert!(
            line.contains("[WARN]") || line.contains("[ERROR]"),
            "an Info entry was shown: {line}"
        );
    }
}

// ---------------------------------------------------------------------------
// FormulaUnavailable: Info, per numeric cell — counted, never printed
// ---------------------------------------------------------------------------

#[test]
fn formula_unavailable_reaches_the_info_count_but_neither_renderer() {
    let (old, new) = numeric_pair();
    let d = compare_bytes(&old, &new).unwrap();

    // Sheet-level `formula_unavailable` per numeric cell on each side, plus the one
    // workbook-level coverage note. The exact number is reported in the review.
    let unavailable = d.sheets[0]
        .diagnostics
        .iter()
        .filter(|x| x.kind.code() == "formula_unavailable")
        .count();
    assert!(unavailable >= NUMERIC_CELLS, "got {unavailable}");
    assert_eq!(
        d.summary.diagnostics.info,
        d.diagnostics.len() + d.sheets[0].diagnostics.len(),
        "every Info diagnostic, at both levels, is counted"
    );
    eprintln!(
        "[measured] numeric fixture, {NUMERIC_CELLS} cells/side: summary.diagnostics.info = {}, \
         of which formula_unavailable = {unavailable}",
        d.summary.diagnostics.info
    );

    // ... and neither renderer says a word about them.
    let unified = render_unified(&d);
    assert!(
        !unified.contains("# Diagnostics"),
        "Info reached the unified output: {unified}"
    );
    assert!(!unified.contains("formula_unavailable"));
    let summary = render_summary(&d);
    assert!(
        !summary.contains("diagnostics:"),
        "Info reached the summary line: {summary}"
    );
}
