//! `FormulaChange::difference` (formulas-move-with-their-rows/02).
//!
//! The annotation says why a formula's two texts differ. It never suppresses: every test that asserts an
//! annotation also asserts the count of changes reported, because the count is what shows nothing was hidden.

use rust_xlsxwriter::{Formula, Workbook};
use sheets_diff::options::AlignmentMode;
use sheets_diff::{
    CellDiff, DiffOptions, FormulaDifference, WorkbookDiff, compare_bytes,
    compare_bytes_with_options,
};

use FormulaDifference::{
    ExplainedByRowMapping, NoRowMovement, NotDetermined, NotExplainedByRowMapping,
};

/// One row: its key in column A, and an optional formula in column D (with a cached result).
struct Row<'a> {
    key: &'a str,
    formula: Option<String>,
}

fn row<'a>(key: &'a str, formula: Option<&str>) -> Row<'a> {
    Row {
        key,
        formula: formula.map(String::from),
    }
}

fn workbook(rows: &[Row<'_>], names: &[&str]) -> Vec<u8> {
    let mut wb = Workbook::new();
    {
        let ws = wb.add_worksheet();
        for (i, r) in rows.iter().enumerate() {
            let i = i as u32;
            ws.write_string(i, 0, r.key).unwrap();
            // A value that belongs to the row's key, not its position, so a moved row's value does not move.
            let n: f64 = r.key.trim_start_matches('k').parse().unwrap_or(0.0);
            ws.write_number(i, 2, n * 10.0).unwrap();
            if let Some(f) = &r.formula {
                ws.write_formula(i, 3, Formula::new(f).set_result("2"))
                    .unwrap();
            }
        }
    }
    for n in names {
        wb.define_name(*n, "=Sheet1!$A$1").unwrap();
    }
    wb.save_to_buffer().unwrap()
}

fn keyed() -> DiffOptions {
    DiffOptions::builder()
        .alignment(AlignmentMode::RowKey { columns: vec![1] })
        .build()
        .unwrap()
}

fn formula_changes(d: &WorkbookDiff) -> Vec<&CellDiff> {
    d.sheets[0]
        .cell_diffs
        .iter()
        .filter(|c| c.formula.is_some())
        .collect()
}

fn differences(d: &WorkbookDiff) -> Vec<FormulaDifference> {
    formula_changes(d)
        .iter()
        .map(|c| c.formula.as_ref().unwrap().difference)
        .collect()
}

/// Ten keyed rows, formula `=C{r}*2` on each, and the same with a row inserted at row 3.
fn shifted_pair(
    old_formula: impl Fn(usize) -> String,
    new_formula: impl Fn(usize) -> String,
) -> (Vec<u8>, Vec<u8>) {
    let keys: Vec<String> = (1..=8).map(|i| format!("k{i}")).collect();
    let old: Vec<Row<'_>> = keys
        .iter()
        .enumerate()
        .map(|(i, k)| row(k, Some(&old_formula(i + 1))))
        .collect();
    let mut new: Vec<Row<'_>> = Vec::new();
    for (i, k) in keys.iter().enumerate() {
        if i == 2 {
            new.push(row("inserted", None));
        }
        let new_row = new.len() + 1;
        new.push(row(k, Some(&new_formula(new_row))));
    }
    (workbook(&old, &[]), workbook(&new, &[]))
}

// ---------------------------------------------------------------------------
// Test 1: the measurement, as a test
// ---------------------------------------------------------------------------

/// 200 rows keyed on column A, one row inserted at row 3, column D `=C{r}*2`. Every moved row's formula is
/// reported as changed, and every one of those is explained. **Both halves are asserted**: the count is what
/// proves nothing was suppressed.
#[test]
fn the_200_row_measurement_reports_everything_and_explains_the_moved_rows() {
    let keys: Vec<String> = (1..=200).map(|i| format!("k{i}")).collect();
    let old: Vec<Row<'_>> = keys
        .iter()
        .enumerate()
        .map(|(i, k)| row(k, Some(&format!("=C{}*2", i + 1))))
        .collect();
    let mut new: Vec<Row<'_>> = Vec::new();
    for (i, k) in keys.iter().enumerate() {
        if i == 2 {
            new.push(row("inserted", Some(&format!("=C{}*2", new.len() + 1))));
        }
        new.push(row(k, Some(&format!("=C{}*2", new.len() + 1))));
    }
    let d = compare_bytes_with_options(workbook(&old, &[]), workbook(&new, &[]), keyed()).unwrap();

    let diffs = differences(&d);
    println!(
        "200-row: {} formula changes; difference tally follows",
        diffs.len()
    );
    let explained = diffs
        .iter()
        .filter(|x| **x == ExplainedByRowMapping)
        .count();
    let other = diffs.len() - explained;
    println!("  ExplainedByRowMapping: {explained}, any other: {other}");
    // Half one, the count: all 198 moved rows (old rows 3..=200) are still reported, plus the inserted row's
    // formula, which is new. 199, exactly what 3.5.0 reports.
    assert_eq!(diffs.len(), 199);
    assert_eq!(d.summary.formulas_changed, 199);
    // Half two, the annotation: every moved row is explained, and the one that is not is the inserted row.
    assert_eq!(explained, 198);
    let inserted: Vec<_> = formula_changes(&d)
        .into_iter()
        .filter(|c| c.formula.as_ref().unwrap().difference != ExplainedByRowMapping)
        .collect();
    assert_eq!(inserted.len(), 1);
    assert_eq!(
        inserted[0].formula.as_ref().unwrap().difference,
        NoRowMovement
    );
}

// ---------------------------------------------------------------------------
// Tests 2 and 3: a genuine edit is not explained
// ---------------------------------------------------------------------------

/// `=C5*2` becomes `=C5*3` on the row that moved from 5 to 6.
#[test]
fn a_genuine_edit_across_a_shift_is_not_explained() {
    let (old, new) = shifted_pair(
        |r| format!("=C{r}*2"),
        |r| {
            if r == 6 {
                "=C5*3".into()
            } else {
                format!("=C{}*2", r - 1)
            }
        },
    );
    let d = compare_bytes_with_options(&old, &new, keyed()).unwrap();
    let edited: Vec<_> = formula_changes(&d)
        .into_iter()
        .filter(|c| c.address.row == 5)
        .collect();
    assert_eq!(edited.len(), 1);
    assert_eq!(
        edited[0].formula.as_ref().unwrap().difference,
        NotExplainedByRowMapping
    );
}

/// An edit and a shift together: `=C5*2` at old row 5 becomes `=C6*3` at new row 6. Mapping gives `=C6*2`, which
/// is not `=C6*3`. A careless implementation maps and then compares loosely.
#[test]
fn an_edit_and_a_shift_together_is_not_explained() {
    let (old, new) = shifted_pair(
        |r| format!("=C{r}*2"),
        |r| {
            if r == 6 {
                "=C6*3".into()
            } else {
                format!("=C{r}*2")
            }
        },
    );
    let d = compare_bytes_with_options(&old, &new, keyed()).unwrap();
    let at = formula_changes(&d)
        .into_iter()
        .find(|c| c.address.row == 5)
        .unwrap();
    assert_eq!(
        at.formula.as_ref().unwrap().difference,
        NotExplainedByRowMapping
    );
    // The neighbours that only moved are explained, and every change is still reported.
    let n_explained = differences(&d)
        .iter()
        .filter(|x| **x == ExplainedByRowMapping)
        .count();
    assert!(n_explained >= 1);
    assert_eq!(formula_changes(&d).len(), differences(&d).len());
}

// ---------------------------------------------------------------------------
// Test 4: positional
// ---------------------------------------------------------------------------

#[test]
fn positional_gives_no_row_movement_for_every_formula_change() {
    let (old, new) = shifted_pair(|r| format!("=C{r}*2"), |r| format!("=C{r}*2"));
    let d = compare_bytes(&old, &new).unwrap();
    let diffs = differences(&d);
    assert!(
        !diffs.is_empty(),
        "the fixture must produce formula changes"
    );
    assert!(diffs.iter().all(|x| *x == NoRowMovement), "{diffs:?}");
}

// ---------------------------------------------------------------------------
// Test 5: each declined class gives NotDetermined, named for the class
// ---------------------------------------------------------------------------

/// Old row 5 holds `old`, and its pair at new row 6 holds `new`; the texts differ so the change is reported.
fn declined_at_row_5(old: &str, new: &str, names: &[&str]) -> FormulaDifference {
    let keys: Vec<String> = (1..=8).map(|i| format!("k{i}")).collect();
    let old_rows: Vec<Row<'_>> = keys
        .iter()
        .enumerate()
        .map(|(i, k)| row(k, (i == 4).then_some(old)))
        .collect();
    let mut new_rows: Vec<Row<'_>> = Vec::new();
    for (i, k) in keys.iter().enumerate() {
        if i == 2 {
            new_rows.push(row("inserted", None));
        }
        new_rows.push(row(k, (i == 4).then_some(new)));
    }
    let d = compare_bytes_with_options(
        workbook(&old_rows, names),
        workbook(&new_rows, names),
        keyed(),
    )
    .unwrap();
    let hits: Vec<_> = formula_changes(&d)
        .into_iter()
        .filter(|c| c.address.row == 5)
        .collect();
    assert_eq!(
        hits.len(),
        1,
        "the change must be reported: {:?}",
        formula_changes(&d)
    );
    hits[0].formula.as_ref().unwrap().difference
}

#[test]
fn a_string_literal_is_not_determined() {
    assert_eq!(
        declined_at_row_5(r#"=IF(A5="x",1,2)"#, r#"=IF(A6="x",1,2)"#, &[]),
        NotDetermined
    );
}

#[test]
fn a_sheet_qualifier_is_not_determined() {
    assert_eq!(
        declined_at_row_5("=Sheet2!C5", "=Sheet2!C6", &[]),
        NotDetermined
    );
}

#[test]
fn a_structured_reference_is_not_determined() {
    assert_eq!(
        declined_at_row_5("=SUM(Table1[Amount])", "=SUM(Table1[Amount ])", &[]),
        NotDetermined
    );
}

/// Old row 9 does not exist in the new sheet's pairing (there are only 8 old rows), so it is not in `matched`.
#[test]
fn a_reference_to_an_unpaired_row_is_not_determined() {
    assert_eq!(declined_at_row_5("=C9*2", "=C8*2", &[]), NotDetermined);
}

/// A declared name refuses, even one shaped like a cell reference; the same formula with no such name declared
/// maps. The pair is the point: the names come from the file.
#[test]
fn a_defined_name_is_not_determined() {
    assert_eq!(
        declined_at_row_5("=TaxRate*C5", "=TaxRate*C6", &["TaxRate"]),
        NotDetermined
    );
    // Shaped like a cell reference (`TAX7`). Declared: declined. Not declared: it is read as a reference and maps.
    assert_eq!(
        declined_at_row_5("=TAX7*C5", "=TAX8*C6", &["TAX7"]),
        NotDetermined
    );
    assert_eq!(
        declined_at_row_5("=TAX7*C5", "=TAX8*C6", &[]),
        ExplainedByRowMapping
    );
}

/// A formula added or removed has nothing to map.
#[test]
fn a_formula_added_or_removed_is_not_explained() {
    let keys: Vec<String> = (1..=4).map(|i| format!("k{i}")).collect();
    let old: Vec<Row<'_>> = keys.iter().map(|k| row(k, None)).collect();
    let new: Vec<Row<'_>> = keys
        .iter()
        .enumerate()
        .map(|(i, k)| row(k, (i == 1).then_some("=C2*2")))
        .collect();
    let d = compare_bytes_with_options(workbook(&old, &[]), workbook(&new, &[]), keyed()).unwrap();
    let diffs = differences(&d);
    assert_eq!(diffs, vec![NotExplainedByRowMapping]);
    assert!(
        formula_changes(&d)[0]
            .formula
            .as_ref()
            .unwrap()
            .old
            .is_none()
    );
}

// ---------------------------------------------------------------------------
// Test 6: NotDetermined is not NotExplainedByRowMapping
// ---------------------------------------------------------------------------

/// Two sheets that differ only in whether the formula is understood: the same shape, a changed constant on one
/// and a string literal on the other.
#[test]
fn not_determined_and_not_explained_are_distinguishable() {
    let understood = declined_at_row_5("=C5*3", "=C6*2", &[]);
    let not_understood = declined_at_row_5(r#"=IF(A5="x",C5*3,0)"#, r#"=IF(A6="x",C6*2,0)"#, &[]);
    assert_eq!(understood, NotExplainedByRowMapping);
    assert_eq!(not_understood, NotDetermined);
    assert_ne!(understood, not_understood);
}
