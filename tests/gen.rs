//! Fixture generator for the `sheets-diff` test corpus (RFC-030).
//!
//! Run with:
//!   cargo test -p sheets-diff --test gen -- --nocapture
//!
//! With the serde feature, also writes `expected.json` golden files:
//!   cargo test --features serde --test gen -- --nocapture

mod support;
use support::*;

use std::path::Path;
use sheets_diff::compare_bytes;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn write_scenario(dir: &Path, name: &str, kind: &str, description: &str) {
    let toml = format!(
        "name        = {:?}\nkind        = {:?}\ndescription = {:?}\nnotes       = \"\"\n",
        name, kind, description
    );
    std::fs::write(dir.join("scenario.toml"), toml).unwrap();
}

fn write_fixture_pair(dir: &Path, old: &[u8], new: &[u8]) {
    std::fs::write(dir.join("old.xlsx"), old).unwrap();
    std::fs::write(dir.join("new.xlsx"), new).unwrap();
}

#[cfg(feature = "serde")]
fn write_expected(dir: &Path, diff: &sheets_diff::WorkbookDiff) {
    let json = sheets_diff::output::json::to_json_pretty(diff).unwrap();
    std::fs::write(dir.join("expected.json"), json).unwrap();
}

#[cfg(not(feature = "serde"))]
fn write_expected(_dir: &Path, _diff: &sheets_diff::WorkbookDiff) {}

// ---------------------------------------------------------------------------
// Scenario generators
// ---------------------------------------------------------------------------

#[test]
fn generate_fixtures() {
    let base = Path::new("tests/fixtures/generated");

    // 1. Wide columns — tests A1 encoding through XFD
    {
        let dir = base.join("wide_columns");
        let old = wb_wide_column(0, 16383, "before"); // 0-based col 16383 → 1-based 16384 → XFD
        let new = wb_wide_column(0, 16383, "after");
        write_fixture_pair(&dir, &old, &new);
        write_scenario(&dir, "wide_columns_xfd",
            "regression",
            "Covers A1 addressing through column XFD (column 16384). \
             Tests the v1 address-encoding bug.");
        let diff = compare_bytes(&old, &new).unwrap();
        assert_eq!(diff.sheets[0].cell_diffs[0].address.a1, "XFD1");
        write_expected(&dir, &diff);
        println!("✓ wide_columns");
    }

    // 2. Renamed sheet — verifies cell diffs survive rename detection
    {
        let dir = base.join("renamed_sheet");
        let old = wb_sheets(&[("OldName", &[(0, 0, "before"), (1, 0, "same")])]);
        let new = wb_sheets(&[("NewName", &[(0, 0, "after"),  (1, 0, "same")])]);
        write_fixture_pair(&dir, &old, &new);
        write_scenario(&dir, "renamed_sheet_with_cell_change",
            "feature",
            "A single renamed sheet with one cell changed. Verifies conservative \
             rename detection and cell diff preservation across rename.");
        let diff = compare_bytes(&old, &new).unwrap();
        assert_eq!(diff.summary.sheets_renamed, 1);
        assert_eq!(diff.summary.cells_changed, 1);
        write_expected(&dir, &diff);
        println!("✓ renamed_sheet");
    }

    // 3. Typed values — text vs number vs bool distinctness
    {
        let dir = base.join("typed_values");
        // old: text "100", true, "2024-01-01"
        // new: number 100.0, false, text "2024-01-01"
        let old = wb_strings(&[(0, 0, "100"), (0, 2, "2024-01-01")]);
        let old2 = {
            let _old = old.clone(); // not used directly — building fresh below
            let old_full = {
                use rust_xlsxwriter::Workbook;
                let mut wb = Workbook::new();
                let ws = wb.add_worksheet();
                ws.write_string(0, 0, "100").unwrap();
                ws.write_boolean(0, 1, true).unwrap();
                ws.write_string(0, 2, "2024-01-01").unwrap();
                wb.save_to_buffer().unwrap()
            };
            old_full
        };
        let new2 = {
            use rust_xlsxwriter::Workbook;
            let mut wb = Workbook::new();
            let ws = wb.add_worksheet();
            ws.write_number(0, 0, 100.0).unwrap();
            ws.write_boolean(0, 1, false).unwrap();
            ws.write_string(0, 2, "2024-01-01").unwrap();
            wb.save_to_buffer().unwrap()
        };
        write_fixture_pair(&dir, &old2, &new2);
        write_scenario(&dir, "typed_values_text_vs_number_vs_bool",
            "feature",
            "Text \"100\" and number 100 are different. Bool true→false is a \
             content change. String \"2024-01-01\" unchanged.");
        let diff = compare_bytes(&old2, &new2).unwrap();
        assert_eq!(diff.summary.values_changed, 2); // A1 (text→number) + B1 (true→false)
        write_expected(&dir, &diff);
        println!("✓ typed_values");
    }

    // 4. Formula — formula text change with same cached value
    {
        let dir = base.join("formula");
        let old = wb_with_formula(0, 0, "label", 1, 0, "=1+1");
        let new = wb_with_formula(0, 0, "label", 1, 0, "=2+0");
        write_fixture_pair(&dir, &old, &new);
        write_scenario(&dir, "formula_text_change",
            "feature",
            "Formula text changes (=1+1 → =2+0) with equivalent result. \
             Tests formula-layer detection independent of value change.");
        let diff = compare_bytes(&old, &new).unwrap();
        // Formula change may or may not be detected depending on xlsx writer storing formula text.
        // The fixture is the canonical example regardless.
        write_expected(&dir, &diff);
        println!("✓ formula");
    }

    // 5. Empty sheet — both sides empty
    {
        let dir = base.join("empty_sheet");
        let b = wb_empty();
        write_fixture_pair(&dir, &b, &b);
        write_scenario(&dir, "empty_sheet_identical",
            "edge_case",
            "Both workbooks have a single empty sheet. Result: no diffs.");
        let diff = compare_bytes(&b, &b).unwrap();
        assert_eq!(diff.summary.cells_changed, 0);
        write_expected(&dir, &diff);
        println!("✓ empty_sheet");
    }

    // 6. Sparse range — only a few cells far apart
    {
        let dir = base.join("sparse_range");
        let old = wb_sparse(&[(0, 0, "A1"), (99, 25, "Z100")]);
        let new = wb_sparse(&[(0, 0, "A1_changed"), (99, 25, "Z100")]);
        write_fixture_pair(&dir, &old, &new);
        write_scenario(&dir, "sparse_range_one_change",
            "feature",
            "Large used range with only two populated cells far apart. \
             Only A1 changes; Z100 stays the same.");
        let diff = compare_bytes(&old, &new).unwrap();
        assert_eq!(diff.summary.cells_changed, 1);
        write_expected(&dir, &diff);
        println!("✓ sparse_range");
    }

    // 7. Row insertion cascade — shows positional vs key-aligned diff count
    {
        let dir = base.join("row_insertion_cascade");
        let old = {
            use rust_xlsxwriter::Workbook;
            let mut wb = Workbook::new();
            let ws = wb.add_worksheet();
            for r in 0u32..20 {
                ws.write_string(r, 0, &format!("id_{r}")).unwrap();
                ws.write_string(r, 1, &format!("val_{r}")).unwrap();
            }
            wb.save_to_buffer().unwrap()
        };
        let new = {
            use rust_xlsxwriter::Workbook;
            let mut wb = Workbook::new();
            let ws = wb.add_worksheet();
            ws.write_string(0, 0, "id_inserted").unwrap();
            ws.write_string(0, 1, "val_inserted").unwrap();
            for r in 0u32..20 {
                ws.write_string(r + 1, 0, &format!("id_{r}")).unwrap();
                ws.write_string(r + 1, 1, &format!("val_{r}")).unwrap();
            }
            wb.save_to_buffer().unwrap()
        };
        write_fixture_pair(&dir, &old, &new);
        write_scenario(&dir, "row_insertion_cascade",
            "regression",
            "One row inserted at the top of a 20-row sheet. Positional diff \
             reports all 20 rows as changed (cascade). Row-key alignment should \
             report only the inserted row.");
        let pos_diff = compare_bytes(&old, &new).unwrap();
        // positional: all 20 original rows shift → reported as changed
        assert!(pos_diff.summary.cells_changed >= 20,
            "positional cascade expected, got {}", pos_diff.summary.cells_changed);
        write_expected(&dir, &pos_diff);
        println!("✓ row_insertion_cascade  (positional={} cell diffs)",
            pos_diff.summary.cells_changed);
    }

    println!("\nAll fixtures generated in {}", base.display());
}
