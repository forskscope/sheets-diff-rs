//! Fixture generator for the `sheets-diff` test corpus (RFC-030, RFC-034).
//!
//! Run with:
//!   cargo run --example gen-fixtures
//!
//! This is a plain example, not a test: `cargo test` never runs it, so a
//! checkout only changes the fixture corpus when this is invoked explicitly.
//! Every generated workbook carries a fixed creation timestamp so re-running
//! this generator against unchanged scenario definitions produces
//! byte-identical `.xlsx` files (RFC-034 §5.2).
//!
//! This generator writes `old.xlsx`, `new.xlsx`, and `scenario.toml` only. It
//! does not write `expected.json` and does not depend on `sheets-diff`'s
//! comparison logic at all — see correction request 01 on Handoff 01: a
//! generator that also produces goldens is a second, silent bless path. The
//! only way `expected.json` changes is
//! `BLESS=1 cargo test --features serde -- generated_fixtures_match_golden`
//! (`tests/integration.rs`), which asserts before it ever writes.

use std::path::Path;

use rust_xlsxwriter::{DocProperties, ExcelDateTime, Formula, Workbook};

// ---------------------------------------------------------------------------
// Fixed-timestamp workbook construction
// ---------------------------------------------------------------------------

/// One fixed creation date for every generated fixture, so byte content
/// depends only on the scenario definition below, never on wall-clock time.
fn fixture_properties() -> DocProperties {
    let date = ExcelDateTime::from_ymd(2020, 1, 1).expect("valid fixed fixture date");
    DocProperties::new().set_creation_datetime(&date)
}

fn new_workbook() -> Workbook {
    let mut wb = Workbook::new();
    wb.set_properties(&fixture_properties());
    wb
}

// ---------------------------------------------------------------------------
// Builder helpers
//
// These deliberately duplicate a subset of tests/support.rs rather than
// sharing it — an example cannot depend on the tests/ integration-test
// module without a workspace restructure (RFC-034 §8 already weighed and
// rejected the equivalent xtask option on the same cost/benefit grounds).
// The duplication is not accidental: unlike tests/support.rs's builders,
// every builder here pins fixture_properties() so the committed corpus is
// byte-reproducible. tests/support.rs's builders must NOT gain a pinned
// timestamp — they back ~60 non-corpus tests that don't write tracked files
// and have no reproducibility requirement. If you change one file's builder
// signatures, check whether the other needs the same change.
// ---------------------------------------------------------------------------

fn wb_wide_column(row: u32, col: u16, value: &str) -> Vec<u8> {
    let mut wb = new_workbook();
    let ws = wb.add_worksheet();
    ws.write_string(row, col, value).unwrap();
    wb.save_to_buffer().unwrap()
}

/// One named sheet's cells, as `(name, cells)`.
type SheetSpec<'a> = (&'a str, &'a [(u32, u16, &'a str)]);

fn wb_sheets(sheets: &[SheetSpec]) -> Vec<u8> {
    let mut wb = new_workbook();
    for (name, cells) in sheets {
        let ws = wb.add_worksheet();
        ws.set_name(*name).unwrap();
        for (row, col, val) in *cells {
            ws.write_string(*row, *col, *val).unwrap();
        }
    }
    wb.save_to_buffer().unwrap()
}

fn wb_with_formula(
    value_row: u32,
    value_col: u16,
    value: &str,
    formula_row: u32,
    formula_col: u16,
    formula: &str,
) -> Vec<u8> {
    let mut wb = new_workbook();
    let ws = wb.add_worksheet();
    ws.write_string(value_row, value_col, value).unwrap();
    ws.write_formula(formula_row, formula_col, Formula::new(formula))
        .unwrap();
    wb.save_to_buffer().unwrap()
}

fn wb_empty() -> Vec<u8> {
    let mut wb = new_workbook();
    wb.add_worksheet();
    wb.save_to_buffer().unwrap()
}

fn wb_strings(cells: &[(u32, u16, &str)]) -> Vec<u8> {
    let mut wb = new_workbook();
    let ws = wb.add_worksheet();
    for (row, col, val) in cells {
        ws.write_string(*row, *col, *val).unwrap();
    }
    wb.save_to_buffer().unwrap()
}

// ---------------------------------------------------------------------------
// Scenario output helpers
// ---------------------------------------------------------------------------

fn write_scenario(dir: &Path, name: &str, kind: &str, description: &str) {
    let toml = format!(
        "name        = {:?}\nkind        = {:?}\ndescription = {:?}\nnotes       = \"\"\n",
        name, kind, description
    );
    std::fs::write(dir.join("scenario.toml"), toml).unwrap();
}

fn write_fixture_pair(dir: &Path, old: &[u8], new: &[u8]) {
    std::fs::create_dir_all(dir).unwrap();
    std::fs::write(dir.join("old.xlsx"), old).unwrap();
    std::fs::write(dir.join("new.xlsx"), new).unwrap();
}

// ---------------------------------------------------------------------------
// Scenarios
// ---------------------------------------------------------------------------

fn main() {
    let base = Path::new("tests/fixtures/generated");

    // 1. Wide columns — tests A1 encoding through XFD
    {
        let dir = base.join("wide_columns");
        let old = wb_wide_column(0, 16383, "before"); // 0-based col 16383 → 1-based 16384 → XFD
        let new = wb_wide_column(0, 16383, "after");
        write_fixture_pair(&dir, &old, &new);
        write_scenario(
            &dir,
            "wide_columns_xfd",
            "regression",
            "Covers A1 addressing through column XFD (column 16384). \
             Tests the v1 address-encoding bug.",
        );
        println!("✓ wide_columns");
    }

    // 2. Renamed sheet — verifies cell diffs survive rename detection
    {
        let dir = base.join("renamed_sheet");
        let old = wb_sheets(&[("OldName", &[(0, 0, "before"), (1, 0, "same")])]);
        let new = wb_sheets(&[("NewName", &[(0, 0, "after"), (1, 0, "same")])]);
        write_fixture_pair(&dir, &old, &new);
        write_scenario(
            &dir,
            "renamed_sheet_with_cell_change",
            "feature",
            "A single renamed sheet with one cell changed. Verifies conservative \
             rename detection and cell diff preservation across rename.",
        );
        println!("✓ renamed_sheet");
    }

    // 3. Typed values — text vs number vs bool distinctness
    {
        let dir = base.join("typed_values");
        let old = {
            let mut wb = new_workbook();
            let ws = wb.add_worksheet();
            ws.write_string(0, 0, "100").unwrap();
            ws.write_boolean(0, 1, true).unwrap();
            ws.write_string(0, 2, "2024-01-01").unwrap();
            wb.save_to_buffer().unwrap()
        };
        let new = {
            let mut wb = new_workbook();
            let ws = wb.add_worksheet();
            ws.write_number(0, 0, 100.0).unwrap();
            ws.write_boolean(0, 1, false).unwrap();
            ws.write_string(0, 2, "2024-01-01").unwrap();
            wb.save_to_buffer().unwrap()
        };
        write_fixture_pair(&dir, &old, &new);
        write_scenario(
            &dir,
            "typed_values_text_vs_number_vs_bool",
            "feature",
            "Text \"100\" and number 100 are different. Bool true→false is a \
             content change. String \"2024-01-01\" unchanged.",
        );
        println!("✓ typed_values");
    }

    // 4. Formula — formula text change with same cached value
    {
        let dir = base.join("formula");
        let old = wb_with_formula(0, 0, "label", 1, 0, "=1+1");
        let new = wb_with_formula(0, 0, "label", 1, 0, "=2+0");
        write_fixture_pair(&dir, &old, &new);
        write_scenario(
            &dir,
            "formula_text_change",
            "feature",
            "Formula text changes (=1+1 → =2+0) with equivalent result. \
             Tests formula-layer detection independent of value change.",
        );
        println!("✓ formula");
    }

    // 5. Empty sheet — both sides empty
    {
        let dir = base.join("empty_sheet");
        let b = wb_empty();
        write_fixture_pair(&dir, &b, &b);
        write_scenario(
            &dir,
            "empty_sheet_identical",
            "edge_case",
            "Both workbooks have a single empty sheet. Result: no diffs.",
        );
        println!("✓ empty_sheet");
    }

    // 6. Sparse range — only a few cells far apart
    {
        let dir = base.join("sparse_range");
        let old = wb_strings(&[(0, 0, "A1"), (99, 25, "Z100")]);
        let new = wb_strings(&[(0, 0, "A1_changed"), (99, 25, "Z100")]);
        write_fixture_pair(&dir, &old, &new);
        write_scenario(
            &dir,
            "sparse_range_one_change",
            "feature",
            "Large used range with only two populated cells far apart. \
             Only A1 changes; Z100 stays the same.",
        );
        println!("✓ sparse_range");
    }

    // 7. Row insertion cascade — shows positional vs key-aligned diff count
    {
        let dir = base.join("row_insertion_cascade");
        let old = {
            let mut wb = new_workbook();
            let ws = wb.add_worksheet();
            for r in 0u32..20 {
                ws.write_string(r, 0, format!("id_{r}")).unwrap();
                ws.write_string(r, 1, format!("val_{r}")).unwrap();
            }
            wb.save_to_buffer().unwrap()
        };
        let new = {
            let mut wb = new_workbook();
            let ws = wb.add_worksheet();
            ws.write_string(0, 0, "id_inserted").unwrap();
            ws.write_string(0, 1, "val_inserted").unwrap();
            for r in 0u32..20 {
                ws.write_string(r + 1, 0, format!("id_{r}")).unwrap();
                ws.write_string(r + 1, 1, format!("val_{r}")).unwrap();
            }
            wb.save_to_buffer().unwrap()
        };
        write_fixture_pair(&dir, &old, &new);
        write_scenario(
            &dir,
            "row_insertion_cascade",
            "regression",
            "One row inserted at the top of a 20-row sheet. Positional diff \
             reports all 20 rows as changed (cascade). Row-key alignment should \
             report only the inserted row.",
        );
        println!("✓ row_insertion_cascade");
    }

    println!("\nAll fixtures generated in {}", base.display());
    println!("expected.json is not written here — bless goldens with:");
    println!("  BLESS=1 cargo test --features serde -- generated_fixtures_match_golden");
}
