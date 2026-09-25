//! The builder covers every option — audited by a test, not by a person noticing (M10 units 03 and 08).
//!
//! **This is a `#[cfg(test)]` module inside `src/` on purpose, and the one place the project's rule against
//! inline test modules (readiness finding B2) is knowingly ignored.** The guard below destructures every options
//! struct *exhaustively* — no `..` — so that adding a field to any of them fails to compile until it is listed. Since
//! M10 unit 08 those structs are `#[non_exhaustive]`, and a pattern without `..` is rejected (`E0638`) from any
//! other crate, so the guard only works from inside this one. Moving it here is the price of making options
//! extensible; deleting it, or weakening it to a `..` pattern so it compiles from `tests/`, would silently remove the
//! guarantee and leave a test that looks like a guard.
//!
//! It works in three layers:
//!
//! 1. `leaves()` destructures **every** options struct *exhaustively* — no `..`.
//! 2. `SETTERS` names one builder call per leaf. A test asserts the two lists are the same
//!    nineteen names, so a leaf that is listed but has no setter cannot hide.
//! 3. Each setter is applied on its own to a default builder and the result compared with the
//!    default: **exactly its own leaf must change, and no other.** That is what "a method of
//!    its own" means — a whole-struct setter would move several leaves, or none it named.
//!
//! One leaf cannot be moved to a non-default value, for the reason stated where it is listed
//! (`execution.mode`: `ExecutionMode` has one variant). For it the test asserts the setter is
//! accepted and changes nothing else.

use std::collections::{BTreeMap, BTreeSet};

use crate::options::{
    AlignmentMode, ComparisonOptions, DateComparePolicy, DiagnosticOptions, DiffOptions,
    DiffOptionsBuilder, ExecutionMode, ExecutionOptions, FormulaCompareMode, Limits,
    MatchingOptions, NumberComparePolicy, NumericTypePolicy, OutputOptions, SheetMatchingMode,
    TypeMismatchPolicy, ValueCompareOptions,
};
use crate::{ObjectCompareMode, Severity};

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
/// The one `false` row is the exception the module comment names: `execution.mode` —
/// `ExecutionMode` has a single variant, `Sequential`, so there is no other value to set.
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
fn the_leaves_are_exactly_the_ones_with_a_setter() {
    let leaf_names: BTreeSet<&str> = default_leaves().keys().copied().collect();
    let setter_names: BTreeSet<&str> = SETTERS.iter().map(|(n, _, _)| *n).collect();
    assert_eq!(leaf_names.len(), 19, "the options tree has 19 leaves");
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
fn all_setters_together_configure_every_movable_leaf() {
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
        "18 of the 19 leaves can be moved off their default"
    );
}
