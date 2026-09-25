//! `DiffOptionsBuilder::min_severity` and `::alignment` (M8 unit 07).
//!
//! `DiffOptions::builder()` is the entry point the crate's documentation points
//! at, and until this unit it could not set two of the options a caller reaches
//! for: the diagnostic filter and the alignment mode. These tests pin the
//! property that matters — a comparison configured **through the builder** is the
//! same comparison as one configured by assigning the public fields — not merely
//! that a setter stores its argument.

mod support;
use support::{wb_numbers, wb_strings};

use sheets_diff::options::AlignmentMode;
use sheets_diff::{
    DiffOptions, Severity, SheetMatchingMode, WorkbookDiff, compare_bytes,
    compare_bytes_with_options,
};

// ---------------------------------------------------------------------------
// Fixtures, built in-test
// ---------------------------------------------------------------------------

/// A key column (A) and a value column (B). The new side gains a row at the top,
/// so `Positional` sees every row shifted and `RowKey` on column A sees one
/// inserted row and nothing else.
fn shifted_pair() -> (Vec<u8>, Vec<u8>) {
    (
        wb_strings(&[
            (0, 0, "k1"),
            (0, 1, "a"),
            (1, 0, "k2"),
            (1, 1, "b"),
            (2, 0, "k3"),
            (2, 1, "c"),
        ]),
        wb_strings(&[
            (0, 0, "k0"),
            (0, 1, "z"),
            (1, 0, "k1"),
            (1, 1, "a"),
            (2, 0, "k2"),
            (2, 1, "b"),
            (3, 0, "k3"),
            (3, 1, "c"),
        ]),
    )
}

/// Two rows share the key `dup`: under `RowKey` alignment this raises the
/// sheet-level `Warning` `duplicate_alignment_key`; under `Positional` it raises
/// nothing.
fn duplicate_key_pair() -> (Vec<u8>, Vec<u8>) {
    (
        wb_strings(&[(0, 0, "dup"), (1, 0, "dup"), (2, 0, "unique")]),
        wb_strings(&[(0, 0, "dup"), (1, 0, "unique")]),
    )
}

/// Numeric cells with no formula: each raises a sheet-level `Info`, plus the
/// workbook-level `Info` coverage note. Nothing here is above `Info`.
fn numeric_pair() -> (Vec<u8>, Vec<u8>) {
    let cells = |offset: f64| -> Vec<(u32, u16, f64)> {
        (0..10u32).map(|r| (r, 0u16, r as f64 + offset)).collect()
    };
    (wb_numbers(&cells(0.0)), wb_numbers(&cells(0.5)))
}

/// A named fixture pair and the severity filter to run it under.
type Case = (&'static str, (Vec<u8>, Vec<u8>), Option<Severity>);

fn diagnostic_count(d: &WorkbookDiff) -> usize {
    d.diagnostics.len() + d.sheets.iter().map(|s| s.diagnostics.len()).sum::<usize>()
}

// ---------------------------------------------------------------------------
// The property: builder-configured == field-configured
// ---------------------------------------------------------------------------

/// The same configuration, set both ways. The field-assignment side is written
/// out independently of the builder — it is the reference, not a helper the
/// builder shares.
fn by_builder(alignment: AlignmentMode, min: Option<Severity>) -> DiffOptions {
    DiffOptions::builder()
        .alignment(alignment)
        .min_severity(min)
        .build()
        .unwrap()
}

fn by_fields(alignment: AlignmentMode, min: Option<Severity>) -> DiffOptions {
    let mut opts = DiffOptions::default();
    opts.matching.alignment = alignment;
    opts.diagnostics.min_severity = min;
    opts
}

fn row_key() -> AlignmentMode {
    AlignmentMode::RowKey { columns: vec![1] }
}

#[test]
fn a_builder_configured_comparison_equals_a_field_configured_one() {
    // Each case names a configuration under which the two options *do
    // something*, so equality is not the trivial equality of two identical
    // defaults. The non-vacuity checks below the loop prove that.
    let cases: Vec<Case> = vec![
        ("dup key, keep all", duplicate_key_pair(), None),
        (
            "dup key, >= Warning",
            duplicate_key_pair(),
            Some(Severity::Warning),
        ),
        (
            "dup key, >= Error",
            duplicate_key_pair(),
            Some(Severity::Error),
        ),
        (
            "shifted rows, >= Warning",
            shifted_pair(),
            Some(Severity::Warning),
        ),
        ("shifted rows, keep all", shifted_pair(), None),
    ];

    for (name, (old, new), min) in cases {
        let built = compare_bytes_with_options(&old, &new, by_builder(row_key(), min)).unwrap();
        let assigned = compare_bytes_with_options(&old, &new, by_fields(row_key(), min)).unwrap();
        assert_eq!(built, assigned, "case `{name}`: the two paths diverged");
    }

    // Non-vacuity 1: `alignment` is doing something — RowKey differs from
    // Positional on the shifted pair.
    let (old, new) = shifted_pair();
    let keyed = compare_bytes_with_options(&old, &new, by_builder(row_key(), None)).unwrap();
    let positional = compare_bytes(&old, &new).unwrap();
    assert_ne!(keyed, positional, "RowKey must change the result");

    // Non-vacuity 2: `min_severity` is doing something — the filter changes the
    // result on the duplicate-key pair, where a Warning exists to be dropped.
    let (old, new) = duplicate_key_pair();
    let kept = compare_bytes_with_options(&old, &new, by_builder(row_key(), None)).unwrap();
    let dropped =
        compare_bytes_with_options(&old, &new, by_builder(row_key(), Some(Severity::Error)))
            .unwrap();
    assert_ne!(kept, dropped, "min_severity must change the result");
    assert!(diagnostic_count(&kept) > diagnostic_count(&dropped));
}

// ---------------------------------------------------------------------------
// min_severity(None) — "say the default"
// ---------------------------------------------------------------------------

#[test]
fn min_severity_none_is_accepted_and_behaves_as_the_default() {
    let (old, new) = numeric_pair();
    let default = compare_bytes(&old, &new).unwrap();
    assert!(
        diagnostic_count(&default) > 0,
        "control: this fixture must produce diagnostics for the default to be observable"
    );

    let opts = DiffOptions::builder().min_severity(None).build().unwrap();
    assert!(opts.diagnostics.min_severity.is_none());
    let said_none = compare_bytes_with_options(&old, &new, opts).unwrap();
    assert_eq!(said_none, default);
}

#[test]
fn min_severity_none_undoes_an_earlier_filter() {
    // The reason the setter takes `Option`: a caller must be able to *say* None,
    // e.g. when layering options over a base configuration.
    let (old, new) = numeric_pair();
    let opts = DiffOptions::builder()
        .min_severity(Some(Severity::Error))
        .min_severity(None)
        .build()
        .unwrap();
    let d = compare_bytes_with_options(&old, &new, opts).unwrap();
    assert_eq!(d, compare_bytes(&old, &new).unwrap());
}

// ---------------------------------------------------------------------------
// alignment — reaches the alignment machinery
// ---------------------------------------------------------------------------

#[test]
fn the_alignment_setter_reaches_alignment() {
    let (old, new) = shifted_pair();

    // Positional: the insertion shifts every row, so no alignment is computed
    // and the cascade shows.
    let positional = compare_bytes(&old, &new).unwrap();
    assert!(positional.sheets[0].alignment_summary.is_none());

    // RowKey via the builder: alignment ran, matched the three original rows and
    // saw one insertion.
    let opts = DiffOptions::builder().alignment(row_key()).build().unwrap();
    let keyed = compare_bytes_with_options(&old, &new, opts).unwrap();
    let summary = keyed.sheets[0]
        .alignment_summary
        .as_ref()
        .expect("RowKey alignment must have run");
    assert_eq!(summary.matched_rows, 3);
    assert_eq!(summary.inserted_rows, 1);
    assert_eq!(summary.removed_rows, 0);
    assert!(
        keyed.summary.cells_changed < positional.summary.cells_changed,
        "keyed={} positional={}",
        keyed.summary.cells_changed,
        positional.summary.cells_changed
    );
}

#[test]
fn the_alignment_setter_leaves_its_sibling_alone() {
    // `MatchingOptions` holds two fields. A setter for one must not disturb the
    // other, whichever order they are called in — the property a whole-struct
    // setter would lose.
    let a = DiffOptions::builder()
        .sheet_matching(SheetMatchingMode::ExactNameOnly)
        .alignment(row_key())
        .build()
        .unwrap();
    let b = DiffOptions::builder()
        .alignment(row_key())
        .sheet_matching(SheetMatchingMode::ExactNameOnly)
        .build()
        .unwrap();
    for opts in [a, b] {
        assert_eq!(
            opts.matching.sheet_matching,
            SheetMatchingMode::ExactNameOnly
        );
        assert!(matches!(
            opts.matching.alignment,
            AlignmentMode::RowKey { ref columns } if columns == &[1]
        ));
    }
}
