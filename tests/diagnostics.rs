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
use support::{wb_formula_and_plain_numbers, wb_numbers, wb_strings};

use sheets_diff::options::AlignmentMode;
use sheets_diff::output::text::{render_summary, render_unified};
use sheets_diff::{DiffOptions, Severity, WorkbookDiff, compare_bytes, compare_bytes_with_options};

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
        .alignment(AlignmentMode::RowKey { columns: vec![1] })
        .build()
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

/// Numeric cells with no formula anywhere in the sheet. **This is f135's own regression
/// shape**: through 3.2.0, every one of these raised a sheet-level `Info` `formula_unavailable`
/// — a 20,000x10 sheet of exactly this shape measured 400,000 of them and a 158.9 MiB result,
/// from a guard (`has_formulas`) that actually meant "the formula pass completed without
/// error", true for essentially every sheet including this one. Fixed: a sheet with no
/// formulas now emits zero.
const NUMERIC_CELLS: usize = 40;
fn numeric_pair() -> (Vec<u8>, Vec<u8>) {
    let cells = |offset: f64| -> Vec<(u32, u16, f64)> {
        (0..NUMERIC_CELLS as u32)
            .map(|r| (r, 0u16, r as f64 + offset))
            .collect()
    };
    (wb_numbers(&cells(0.0)), wb_numbers(&cells(0.5)))
}

/// A sheet that genuinely has formulas, mixed with `N` plain numeric cells that have none --
/// the shape `formula_unavailable` exists for. One formula cell (column 0), `n` plain numeric
/// cells (column 1..), each side identical so the comparison itself reports no changes and
/// only the diagnostics are under test.
fn formula_and_plain_pair(n: u32) -> (Vec<u8>, Vec<u8>) {
    let make = || {
        let formulas: Vec<(u32, u16, &str, f64)> = vec![(0, 0, "=1+1", 2.0)];
        let plain: Vec<(u32, u16, f64)> = (0..n).map(|r| (r, 1u16, r as f64)).collect();
        wb_formula_and_plain_numbers(&formulas, &plain)
    };
    (make(), make())
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
    s.warnings + s.info
}

// ===========================================================================
// min_severity — a collection filter, observed with no renderer involved
// ===========================================================================

#[test]
fn min_severity_none_collects_every_diagnostic() {
    // The control: proves the filter is not always on. Needs a genuine sheet-level Info
    // alongside the workbook-level one; `numeric_pair` (no formulas at all) no longer produces
    // one since f135 -- that used to be the bug this fixture exercised by accident.
    let (old, new) = formula_and_plain_pair(5);
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
    // A genuine sheet-level Info to drop, not `numeric_pair` (which produces none since f135
    // and would make the sheet-level assertion below trivially true either way).
    let (old, new) = formula_and_plain_pair(5);
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

/// `min_severity`'s `>=` filter depends on the derived order, and there are exactly two levels.
#[test]
fn severity_is_ordered_info_below_warning() {
    assert!(Severity::Info < Severity::Warning);
    assert!(Severity::Warning >= Severity::Info);
    assert_eq!(Severity::Info.max(Severity::Warning), Severity::Warning);
}

#[test]
fn min_severity_warning_keeps_warnings_and_info_is_the_floor() {
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

    // ... and `Info` is the floor: asking for it keeps everything, exactly as `None` does.
    let all = compare_bytes_with_options(&old, &new, row_key_options(Some(Severity::Info), None))
        .unwrap();
    let none = compare_bytes_with_options(&old, &new, row_key_options(None, None)).unwrap();
    assert_eq!(all, none);
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
    // A genuine formula-bearing sheet: workbook-level Info (coverage note) + sheet-level Info
    // (formula_unavailable, once per side since f135 -- not the all-numeric, no-formula shape,
    // which correctly produces none of the sheet-level kind any more).
    let (old, new) = formula_and_plain_pair(5);
    let d = compare_bytes(&old, &new).unwrap();
    assert!(
        !d.diagnostics.is_empty() && !d.sheets[0].diagnostics.is_empty(),
        "precondition"
    );
    assert_eq!(summary_total(&d) as u64, d.metrics.diagnostics_emitted);

    // And with a filter applied, they still agree with each other and with the vectors.
    let (old, new) = duplicate_key_pair();
    for min in [None, Some(Severity::Info), Some(Severity::Warning)] {
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
    assert!(s.contains("diagnostics: 1 warning(s)"), "got: {s}");
    assert!(
        !s.contains("error"),
        "the summary line has no error count (there is no error severity): {s}"
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
// FormulaUnavailable: Info, once per sheet with a count — never printed (f135)
// ---------------------------------------------------------------------------

/// **The f135 regression.** A sheet with no formulas anywhere emits zero
/// `formula_unavailable` — not one per numeric cell, which is what every version through
/// 3.2.0 did (`has_formulas` meant "the formula pass completed without error", true here).
#[test]
fn no_formula_numeric_sheet_emits_zero_formula_unavailable() {
    let (old, new) = numeric_pair();
    let d = compare_bytes(&old, &new).unwrap();

    let unavailable = d.sheets[0]
        .diagnostics
        .iter()
        .filter(|x| x.kind.code() == "formula_unavailable")
        .count();
    assert_eq!(
        unavailable, 0,
        "a sheet with no formulas must not raise formula_unavailable at all: {:?}",
        d.sheets[0].diagnostics
    );
}

/// A sheet that genuinely has a formula still reports — a guard fix that silences the true
/// case too would be a worse defect than the one being fixed. One `Info` per side (old read +
/// new read each contribute their own), not one per plain numeric cell: the message carries
/// the count instead. Neither renderer prints `Info` regardless (O3's existing rule).
#[test]
fn formula_bearing_sheet_still_reports_once_per_side_with_a_count() {
    let (old, new) = formula_and_plain_pair(40);
    let d = compare_bytes(&old, &new).unwrap();

    let unavailable: Vec<_> = d.sheets[0]
        .diagnostics
        .iter()
        .filter(|x| x.kind.code() == "formula_unavailable")
        .collect();
    assert_eq!(
        unavailable.len(),
        2,
        "one per side (old read, new read), not one per cell: {:?}",
        unavailable
    );
    for diag in &unavailable {
        assert_eq!(diag.severity, Severity::Info);
        assert!(
            diag.location.address.is_none(),
            "not about one cell any more: {:?}",
            diag.location
        );
        assert_eq!(diag.location.sheet_name.as_deref(), Some("Sheet1"));
        assert!(
            diag.message.contains("40"),
            "message should carry the count: {}",
            diag.message
        );
    }
    assert_eq!(
        d.summary.diagnostics.info,
        d.diagnostics.len() + d.sheets[0].diagnostics.len(),
        "every Info diagnostic, at both levels, is counted"
    );

    // ... and neither renderer says a word about them (O3's existing rule, unaffected).
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

/// The count is bounded by sheets (well, by sides of a sheet), not by cells: a sheet with
/// 4,000 plain numeric cells produces exactly as many `formula_unavailable` diagnostics as one
/// with 40 -- two, one per side -- even though the count *inside* each message's text differs.
#[test]
fn formula_unavailable_diagnostic_count_does_not_grow_with_plain_numeric_cells() {
    let count_for = |n: u32| {
        let (old, new) = formula_and_plain_pair(n);
        let d = compare_bytes(&old, &new).unwrap();
        d.sheets[0]
            .diagnostics
            .iter()
            .filter(|x| x.kind.code() == "formula_unavailable")
            .count()
    };
    let small = count_for(40);
    let large = count_for(4_000);
    assert_eq!(small, 2, "got {small}");
    assert_eq!(
        large, small,
        "100x the plain numeric cells must not multiply the diagnostic count"
    );
}
