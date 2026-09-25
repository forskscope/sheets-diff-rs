//! The builder covers every option — audited by a test, not by a person noticing (M10 unit 03).
//!
//! `DiffOptions` documents `builder()` as the way to configure it. This file is the standing
//! audit behind that sentence. It works in three layers:
//!
//! 1. `leaves()` destructures **every** options struct *exhaustively* — no `..` — so adding a
//!    field to any of them is a compile error here until it is listed.
//! 2. `SETTERS` names one builder call per leaf. A test asserts the two lists are the same
//!    twenty names, so a leaf that is listed but has no setter cannot hide.
//! 3. Each setter is applied on its own to a default builder and the result compared with the
//!    default: **exactly its own leaf must change, and no other.** That is what "a method of
//!    its own" means — a whole-struct setter would move several leaves, or none it named.
//!
//! Two leaves cannot be moved to a non-default value through `build()`, for reasons that are
//! stated where they are listed (`execution.mode` has one variant; `comparison.format` accepts
//! only `Ignore`). For those the test asserts the setter is accepted and changes nothing else.

mod support;
use support::{patch_xlsx_xml, wb_strings};

use std::collections::{BTreeMap, BTreeSet};

use rust_xlsxwriter::Workbook;
use sheets_diff::options::{
    AlignmentMode, ComparisonOptions, DiagnosticOptions, ExecutionOptions, Limits, MatchingOptions,
    OutputOptions, ValueCompareOptions,
};
use sheets_diff::{
    DateComparePolicy, DiffOptions, DiffOptionsBuilder, ExecutionMode, FormatCompareMode,
    FormulaCompareMode, NumberComparePolicy, NumericTypePolicy, ObjectCompareMode, Severity,
    SheetMatchingMode, SheetsDiffError, TypeMismatchPolicy, WorkbookDiff, compare_bytes,
    compare_bytes_with_options,
};

// ---------------------------------------------------------------------------
// Layer 1 — every leaf, read back
// ---------------------------------------------------------------------------

/// Every leaf option of `DiffOptions`, by path, rendered so it can be compared.
/// The destructuring is exhaustive **on purpose**: a new field fails to compile here.
fn leaves(o: &DiffOptions) -> BTreeMap<&'static str, String> {
    let DiffOptions {
        comparison,
        matching,
        limits,
        execution,
        diagnostics,
        output,
    } = o;
    let ComparisonOptions {
        value,
        formula,
        include_formula_cached_values,
        format,
    } = comparison;
    let ValueCompareOptions {
        number,
        numeric_type,
        date,
        type_mismatch,
    } = value;
    let MatchingOptions {
        sheet_matching,
        alignment,
    } = matching;
    let Limits {
        max_sheets,
        max_cells_read,
        max_cells_compared,
        max_diffs_returned,
        max_alignment_product,
        max_input_bytes,
    } = limits;
    let ExecutionOptions {
        progress,
        cancellation,
        mode,
    } = execution;
    let DiagnosticOptions { min_severity } = diagnostics;
    let OutputOptions { objects } = output;

    let mut m = BTreeMap::new();
    let mut put = |k: &'static str, v: String| {
        assert!(m.insert(k, v).is_none(), "leaf `{k}` listed twice");
    };
    put("comparison.value.number", format!("{number:?}"));
    put("comparison.value.numeric_type", format!("{numeric_type:?}"));
    put("comparison.value.date", format!("{date:?}"));
    put(
        "comparison.value.type_mismatch",
        format!("{type_mismatch:?}"),
    );
    put("comparison.formula", format!("{formula:?}"));
    put(
        "comparison.include_formula_cached_values",
        format!("{include_formula_cached_values:?}"),
    );
    put("comparison.format", format!("{format:?}"));
    put("matching.sheet_matching", format!("{sheet_matching:?}"));
    put("matching.alignment", format!("{alignment:?}"));
    put("limits.max_sheets", format!("{max_sheets:?}"));
    put("limits.max_cells_read", format!("{max_cells_read:?}"));
    put(
        "limits.max_cells_compared",
        format!("{max_cells_compared:?}"),
    );
    put(
        "limits.max_diffs_returned",
        format!("{max_diffs_returned:?}"),
    );
    put(
        "limits.max_alignment_product",
        format!("{max_alignment_product:?}"),
    );
    put("limits.max_input_bytes", format!("{max_input_bytes:?}"));
    // Trait objects have no Debug or Eq: what a caller can observe is whether one is set.
    put("execution.progress", format!("set={}", progress.is_some()));
    put(
        "execution.cancellation",
        format!("set={}", cancellation.is_some()),
    );
    put("execution.mode", format!("{mode:?}"));
    put("diagnostics.min_severity", format!("{min_severity:?}"));
    put("output.objects", format!("{objects:?}"));
    m
}

// ---------------------------------------------------------------------------
// Layer 2 — one builder call per leaf
// ---------------------------------------------------------------------------

type Setter = fn(DiffOptionsBuilder) -> DiffOptionsBuilder;

/// `(leaf, setter, whether the value it sets differs from the default)`.
///
/// The two `false` rows are the exceptions the module comment names:
/// * `execution.mode` — `ExecutionMode` has a single variant, `Sequential`, so there is no
///   other value to set.
/// * `comparison.format` — `validate()` rejects every `FormatCompareMode` except `Ignore`, so
///   `build()` cannot return a non-default one (RFC-037 §3.3 is about that).
const SETTERS: &[(&str, Setter, bool)] = &[
    (
        "comparison.value.number",
        |b| b.number_compare_policy(NumberComparePolicy::AbsoluteTolerance(0.5)),
        true,
    ),
    (
        "comparison.value.numeric_type",
        |b| b.numeric_type_policy(NumericTypePolicy::CompareMathematicalValue),
        true,
    ),
    (
        "comparison.value.date",
        |b| b.date_compare_policy(DateComparePolicy::NormalizeEquivalentDateTimes),
        true,
    ),
    (
        "comparison.value.type_mismatch",
        |b| b.type_mismatch_policy(TypeMismatchPolicy::CompareDisplayString),
        true,
    ),
    (
        "comparison.formula",
        |b| b.formula_compare(FormulaCompareMode::Ignore),
        true,
    ),
    (
        "comparison.include_formula_cached_values",
        |b| b.include_formula_cached_values(false),
        true,
    ),
    (
        "comparison.format",
        |b| b.format_compare(FormatCompareMode::Ignore),
        false,
    ),
    (
        "matching.sheet_matching",
        |b| b.sheet_matching(SheetMatchingMode::ExactNameOnly),
        true,
    ),
    (
        "matching.alignment",
        |b| b.alignment(AlignmentMode::RowKey { columns: vec![2] }),
        true,
    ),
    ("limits.max_sheets", |b| b.max_sheets(7), true),
    (
        "limits.max_cells_read",
        |b| b.max_cells_read(Some(11)),
        true,
    ),
    (
        "limits.max_cells_compared",
        |b| b.max_cells_compared(13),
        true,
    ),
    (
        "limits.max_diffs_returned",
        |b| b.max_diffs_returned(17),
        true,
    ),
    (
        "limits.max_alignment_product",
        |b| b.max_alignment_product(Some(19)),
        true,
    ),
    (
        "limits.max_input_bytes",
        |b| b.max_input_bytes(Some(23)),
        true,
    ),
    ("execution.progress", |b| b.progress(|_event| {}), true),
    ("execution.cancellation", |b| b.cancellation(|| false), true),
    (
        "execution.mode",
        |b| b.execution_mode(ExecutionMode::Sequential),
        false,
    ),
    (
        "diagnostics.min_severity",
        |b| b.min_severity(Some(Severity::Warning)),
        true,
    ),
    (
        "output.objects",
        |b| b.object_mode(ObjectCompareMode::Ignore),
        true,
    ),
];

fn default_leaves() -> BTreeMap<&'static str, String> {
    leaves(&DiffOptions::default())
}

// ---------------------------------------------------------------------------
// Layer 3 — the audit
// ---------------------------------------------------------------------------

#[test]
fn the_twenty_leaves_are_exactly_the_ones_with_a_setter() {
    let leaf_names: BTreeSet<&str> = default_leaves().keys().copied().collect();
    let setter_names: BTreeSet<&str> = SETTERS.iter().map(|(n, _, _)| *n).collect();
    assert_eq!(leaf_names.len(), 20, "the options tree has 20 leaves");
    assert_eq!(
        leaf_names, setter_names,
        "a leaf option and the builder's setter table have drifted apart"
    );
    assert_eq!(
        SETTERS.len(),
        setter_names.len(),
        "a setter is listed twice"
    );
}

#[test]
fn each_setter_changes_its_own_leaf_and_no_other() {
    let base = default_leaves();
    for (leaf, set, differs) in SETTERS {
        let built = set(DiffOptions::builder())
            .build()
            .unwrap_or_else(|e| panic!("`{leaf}`: the setter's value does not build: {e}"));
        let now = leaves(&built);
        let changed: Vec<&str> = base
            .keys()
            .filter(|k| base[*k] != now[*k])
            .copied()
            .collect();
        if *differs {
            assert_eq!(
                changed,
                [*leaf],
                "`{leaf}`'s setter must change that leaf and only that leaf"
            );
        } else {
            assert!(
                changed.is_empty(),
                "`{leaf}`'s setter (which can only set the default) changed {changed:?}"
            );
        }
    }
}

#[test]
fn all_twenty_setters_together_configure_all_twenty_leaves() {
    let mut b = DiffOptions::builder();
    for (_, set, _) in SETTERS {
        b = set(b);
    }
    let now = leaves(&b.build().unwrap());
    let base = default_leaves();
    let changed: BTreeSet<&str> = base
        .keys()
        .filter(|k| base[*k] != now[*k])
        .copied()
        .collect();
    let expected: BTreeSet<&str> = SETTERS
        .iter()
        .filter(|(_, _, differs)| *differs)
        .map(|(n, _, _)| *n)
        .collect();
    assert_eq!(changed, expected);
    assert_eq!(
        changed.len(),
        18,
        "18 leaves can be moved off their default"
    );
}

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
