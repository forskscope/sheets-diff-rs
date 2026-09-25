//! Options as a downstream caller sees them (M10 units 03 and 08).
//!
//! Every options struct is `#[non_exhaustive]`, so from outside the crate — which is where this file is — a struct
//! *literal* does not compile, but **reading and assigning fields does**, and `Default` plus assignment, or the builder,
//! reach every option. These tests are what a downstream caller does. (The coverage audit that needs to look *inside*
//! the structs lives in `src/builder_coverage.rs`, where `#[non_exhaustive]` does not apply.)
//!
//! `..Default::default()` is **not** a construction path from outside the crate: functional update is a struct
//! expression, and it is rejected exactly as a full literal is (`E0639`; the `compile_fail` doctest on each struct
//! pins it). The migration from a literal is the builder, or `Default::default()` followed by field assignment.

mod support;
use support::{patch_xlsx_xml, wb_strings};

use rust_xlsxwriter::Workbook;
use sheets_diff::options::{
    AlignmentMode, ComparisonOptions, DiagnosticOptions, ExecutionOptions, Limits, MatchingOptions,
    OutputOptions, ValueCompareOptions,
};
use sheets_diff::{
    DateComparePolicy, DiffOptions, ExecutionMode, FormulaCompareMode, NumberComparePolicy,
    NumericTypePolicy, ObjectCompareMode, Severity, SheetMatchingMode, SheetsDiffError,
    TypeMismatchPolicy, WorkbookDiff, compare_bytes, compare_bytes_with_options,
};

// ---------------------------------------------------------------------------
// The equivalence property, including the two options this unit adds
// ---------------------------------------------------------------------------

/// A date cell written under the 1900 system, and the same real date under 1904: equal
/// under `NormalizeEquivalentDateTimes`, different under `ExactRepresentation`.
fn epoch_pair() -> (Vec<u8>, Vec<u8>) {
    use rust_xlsxwriter::{ExcelDateTime, Format};
    let base = {
        let mut wb = Workbook::new();
        let ws = wb.add_worksheet();
        let f = Format::new().set_num_format("yyyy-mm-dd");
        ws.write_datetime_with_format(0, 0, ExcelDateTime::from_ymd(2024, 6, 15).unwrap(), &f)
            .unwrap();
        wb.save_to_buffer().unwrap()
    };
    let empty = {
        let mut wb = Workbook::new();
        wb.add_worksheet();
        wb.save_to_buffer().unwrap()
    };
    let added = compare_bytes(&empty, &base).unwrap();
    let serial_1900 = match &added.sheets[0].cell_diffs[0].value.as_ref().unwrap().new {
        sheets_diff::CellValue::DateTime(dt) => dt.serial,
        other => panic!("expected a DateTime, got {other:?}"),
    };
    let serial_1904 = serial_1900 - 1462.0;
    let new = patch_xlsx_xml(&base, "xl/workbook.xml", |xml| {
        xml.replacen("<workbookPr", "<workbookPr date1904=\"1\" ", 1)
    });
    let new = patch_xlsx_xml(&new, "xl/worksheets/sheet1.xml", |xml| {
        xml.replace(&serial_1900.to_string(), &serial_1904.to_string())
    });
    (base, new)
}

fn outcome(r: Result<WorkbookDiff, SheetsDiffError>) -> Result<WorkbookDiff, String> {
    r.map_err(|e| e.to_string())
}

/// `date_compare_policy` — builder-configured equals field-configured, and the option is
/// doing something (the default disagrees).
#[test]
fn date_compare_policy_equals_field_assignment_and_is_not_vacuous() {
    let (old, new) = epoch_pair();
    let built = DiffOptions::builder()
        .date_compare_policy(DateComparePolicy::NormalizeEquivalentDateTimes)
        .build()
        .unwrap();
    let mut assigned = DiffOptions::default();
    assigned.comparison.value.date = DateComparePolicy::NormalizeEquivalentDateTimes;

    let via_builder = compare_bytes_with_options(&old, &new, built).unwrap();
    let via_field = compare_bytes_with_options(&old, &new, assigned).unwrap();
    assert_eq!(via_builder, via_field);

    let default = compare_bytes(&old, &new).unwrap();
    assert_eq!(
        default.summary.values_changed, 1,
        "control: exact comparison sees a change"
    );
    assert_eq!(
        via_builder.summary.values_changed, 0,
        "the policy reconciles the epochs"
    );
    assert_ne!(via_builder, default);
}

/// `max_cells_read` — same property; the two paths produce the same *error* (the limit fires),
/// and the default produces none.
#[test]
fn max_cells_read_equals_field_assignment_and_is_not_vacuous() {
    let old = wb_strings(&[(0, 0, "a"), (1, 0, "b"), (2, 0, "c")]);
    let new = wb_strings(&[(0, 0, "a"), (1, 0, "b"), (2, 0, "z")]);

    let built = DiffOptions::builder()
        .max_cells_read(Some(2))
        .build()
        .unwrap();
    let mut assigned = DiffOptions::default();
    assigned.limits.max_cells_read = Some(2);

    let via_builder = outcome(compare_bytes_with_options(&old, &new, built));
    let via_field = outcome(compare_bytes_with_options(&old, &new, assigned));
    assert_eq!(via_builder, via_field);
    assert!(
        via_builder.is_err(),
        "a bound of 2 must trip on a 3-cell sheet"
    );
    assert!(
        compare_bytes(&old, &new).is_ok(),
        "control: no bound, no error"
    );
}

/// `max_cells_read(None)` is accepted and behaves as the default (unbounded).
#[test]
fn max_cells_read_none_behaves_as_the_default() {
    let old = wb_strings(&[(0, 0, "a"), (1, 0, "b")]);
    let new = wb_strings(&[(0, 0, "a"), (1, 0, "c")]);
    let opts = DiffOptions::builder().max_cells_read(None).build().unwrap();
    assert!(opts.limits.max_cells_read.is_none());
    let said_none = outcome(compare_bytes_with_options(&old, &new, opts));
    assert_eq!(said_none, outcome(compare_bytes(&old, &new)));

    // ... and it undoes an earlier bound.
    let undone = DiffOptions::builder()
        .max_cells_read(Some(1))
        .max_cells_read(None)
        .build()
        .unwrap();
    assert!(undone.limits.max_cells_read.is_none());
}

// ---------------------------------------------------------------------------
// M10 unit 08 — what a downstream caller can still do
// ---------------------------------------------------------------------------

/// Field **assignment and reading** work on every options struct from outside the crate: `#[non_exhaustive]`
/// stops construction by literal, nothing else. Each struct is created with `Default` (or `Limits::hardened()`),
/// every field is assigned a non-default value and read back — so a field made private, or a struct made
/// unreachable, would fail here. Covers all eight structs and all nineteen leaves.
#[test]
fn every_options_struct_can_be_created_by_default_then_read_and_assigned() {
    // Nested, through DiffOptions.
    let mut o = DiffOptions::default();
    o.comparison.value.number = NumberComparePolicy::AbsoluteTolerance(0.25);
    o.comparison.value.numeric_type = NumericTypePolicy::CompareMathematicalValue;
    o.comparison.value.date = DateComparePolicy::NormalizeEquivalentDateTimes;
    o.comparison.value.type_mismatch = TypeMismatchPolicy::CompareDisplayString;
    o.comparison.formula = FormulaCompareMode::Ignore;
    o.comparison.include_formula_cached_values = false;
    o.matching.sheet_matching = SheetMatchingMode::ExactNameOnly;
    o.matching.alignment = AlignmentMode::RowKey { columns: vec![2] };
    o.limits.max_sheets = Some(1);
    o.limits.max_cells_read = Some(2);
    o.limits.max_cells_compared = Some(3);
    o.limits.max_diffs_returned = Some(4);
    o.limits.max_alignment_product = Some(5);
    o.limits.max_input_bytes = Some(6);
    o.execution.progress = Some(Box::new(|_e| {}));
    o.execution.cancellation = Some(Box::new(|| false));
    o.execution.mode = ExecutionMode::Sequential;
    o.diagnostics.min_severity = Some(Severity::Warning);
    o.output.objects = ObjectCompareMode::Ignore;

    assert_eq!(
        o.comparison.value.number,
        NumberComparePolicy::AbsoluteTolerance(0.25)
    );
    assert_eq!(
        o.comparison.value.numeric_type,
        NumericTypePolicy::CompareMathematicalValue
    );
    assert_eq!(
        o.comparison.value.date,
        DateComparePolicy::NormalizeEquivalentDateTimes
    );
    assert_eq!(
        o.comparison.value.type_mismatch,
        TypeMismatchPolicy::CompareDisplayString
    );
    assert_eq!(o.comparison.formula, FormulaCompareMode::Ignore);
    assert!(!o.comparison.include_formula_cached_values);
    assert_eq!(o.matching.sheet_matching, SheetMatchingMode::ExactNameOnly);
    assert!(matches!(&o.matching.alignment, AlignmentMode::RowKey { columns } if columns == &[2]));
    assert_eq!(
        (
            o.limits.max_sheets,
            o.limits.max_cells_read,
            o.limits.max_cells_compared,
            o.limits.max_diffs_returned,
            o.limits.max_alignment_product,
            o.limits.max_input_bytes
        ),
        (Some(1), Some(2), Some(3), Some(4), Some(5), Some(6))
    );
    assert!(o.execution.progress.is_some() && o.execution.cancellation.is_some());
    assert_eq!(o.execution.mode, ExecutionMode::Sequential);
    assert_eq!(o.diagnostics.min_severity, Some(Severity::Warning));
    assert_eq!(o.output.objects, ObjectCompareMode::Ignore);

    // Each of the seven inner structs, on its own: created by `Default`, read, assigned.
    let mut v = ValueCompareOptions::default();
    v.date = DateComparePolicy::NormalizeEquivalentDateTimes;
    assert_eq!(v.date, DateComparePolicy::NormalizeEquivalentDateTimes);
    let mut c = ComparisonOptions::default();
    c.include_formula_cached_values = false;
    assert!(!c.include_formula_cached_values);
    let mut m = MatchingOptions::default();
    m.alignment = AlignmentMode::Positional;
    assert!(matches!(m.alignment, AlignmentMode::Positional));
    let mut l = Limits::hardened();
    l.max_sheets = None;
    assert!(l.max_sheets.is_none() && l.max_cells_read.is_some());
    let mut e = ExecutionOptions::default();
    e.mode = ExecutionMode::Sequential;
    assert_eq!(e.mode, ExecutionMode::Sequential);
    let mut d = DiagnosticOptions::default();
    d.min_severity = Some(Severity::Info);
    assert_eq!(d.min_severity, Some(Severity::Info));
    let mut out = OutputOptions::default();
    out.objects = ObjectCompareMode::Ignore;
    assert_eq!(out.objects, ObjectCompareMode::Ignore);
}

/// **The migration path, tested.** A caller who wrote a struct literal moves to `Default::default()` followed by
/// field assignment, or to the builder; the two produce the same comparison. (Not `..Default::default()`: functional
/// update is a struct expression and is rejected from outside the crate — see the `compile_fail` doctest on each struct.)
/// Non-vacuity: the configuration changes the result (alignment reshapes it; the filter drops `Info`).
#[test]
fn default_plus_assignment_equals_the_builder() {
    let old = wb_strings(&[(0, 0, "k1"), (0, 1, "a"), (1, 0, "k2"), (1, 1, "b")]);
    let new = wb_strings(&[
        (0, 0, "k0"),
        (0, 1, "z"),
        (1, 0, "k1"),
        (1, 1, "a"),
        (2, 0, "k2"),
        (2, 1, "b"),
    ]);

    let built = DiffOptions::builder()
        .alignment(AlignmentMode::RowKey { columns: vec![1] })
        .min_severity(Some(Severity::Warning))
        .max_cells_read(Some(1_000))
        .build()
        .unwrap();
    let mut assigned = DiffOptions::default();
    assigned.matching.alignment = AlignmentMode::RowKey { columns: vec![1] };
    assigned.diagnostics.min_severity = Some(Severity::Warning);
    assigned.limits.max_cells_read = Some(1_000);

    let via_builder = compare_bytes_with_options(&old, &new, built).unwrap();
    let via_fields = compare_bytes_with_options(&old, &new, assigned).unwrap();
    assert_eq!(via_builder, via_fields);
    assert_ne!(
        via_builder,
        compare_bytes(&old, &new).unwrap(),
        "the configuration must change the result, or the equality proves nothing"
    );
}
