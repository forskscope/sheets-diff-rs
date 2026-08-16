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
//! `BLESS=1 cargo test --features serde,chrono -- generated_fixtures_match_golden`
//! (`tests/integration.rs`), which asserts before it ever writes. `serde`
//! and `chrono` together, not `serde` alone, are the canonical feature set
//! goldens are blessed under (RFC-036 Handoff 02 correction C-01) — a
//! date-bearing fixture's exact JSON depends on `chrono` being enabled.

use std::path::Path;

use rust_xlsxwriter::{Chart, ChartType, DocProperties, ExcelDateTime, Format, Formula, Workbook};

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

/// Rewrite one XML entry inside a generated `.xlsx` with `patch` applied to
/// its UTF-8 text content; every other entry is copied through unchanged.
///
/// Duplicated from `tests/support.rs::patch_xlsx_xml` rather than shared
/// (RFC-036 Handoff 03 §1) — this generator deliberately does not depend on
/// anything under `tests/`, the same reason its other builders above
/// duplicate `tests/support.rs` rather than importing it: the fixture bytes
/// must not be influenced by, or coupled to, the code under test. Reused
/// verbatim rather than reimplemented, since re-verifying zip round-trip
/// correctness from scratch would buy nothing. Byte-reproducibility of its
/// output was verified empirically (two runs, 2s apart, byte-identical) —
/// `zip::write::FileOptions::default()`'s timestamp does not vary with
/// wall-clock time in this build configuration; see the Handoff 03 review
/// request for the check.
fn patch_xlsx_xml(
    xlsx_bytes: &[u8],
    entry_path: &str,
    patch: impl Fn(String) -> String,
) -> Vec<u8> {
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(xlsx_bytes)).unwrap();
    let mut out_buf = Vec::new();
    let mut writer = zip::ZipWriter::new(std::io::Cursor::new(&mut out_buf));
    let mut patched = false;

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).unwrap();
        let name = entry.name().to_string();
        let mut contents = Vec::new();
        std::io::Read::read_to_end(&mut entry, &mut contents).unwrap();

        let options: zip::write::FileOptions<()> = zip::write::FileOptions::default();
        writer.start_file(&name, options).unwrap();

        if name == entry_path {
            let text = String::from_utf8(contents).unwrap();
            let patched_text = patch(text);
            std::io::Write::write_all(&mut writer, patched_text.as_bytes()).unwrap();
            patched = true;
        } else {
            std::io::Write::write_all(&mut writer, &contents).unwrap();
        }
    }
    writer.finish().unwrap();

    assert!(patched, "entry {entry_path} not found in xlsx archive");
    out_buf
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

    // -----------------------------------------------------------------------
    // RFC-036 §5.2 matrix — scenarios 1-9 (Handoff 02, rust_xlsxwriter-only)
    // -----------------------------------------------------------------------

    // 8. Row-shifted origin — data block starts at row 5, not row 1.
    {
        let dir = base.join("row_shifted_origin");
        let old = wb_strings(&[(4, 0, "before")]); // row 5 (0-based row 4), col A
        let new = wb_strings(&[(4, 0, "after")]);
        write_fixture_pair(&dir, &old, &new);
        write_scenario(
            &dir,
            "row_shifted_origin",
            "regression",
            "Data block starts at row 5, not row 1 (leading empty rows). \
             Value-only, no formulas. Coverage for coordinate arithmetic \
             that assumes origin row 1 (RFC-036 #1).",
        );
        println!("✓ row_shifted_origin");
    }

    // 9. Formula origin shifted — value and formula ranges both start below
    //    row 1, with a gap between them (also exercises non-contiguous
    //    formula cells).
    {
        let dir = base.join("formula_shifted_origin");
        let old = wb_with_formula(4, 0, "label", 6, 0, "=1+1"); // row5=label, row7=formula
        let new = wb_with_formula(4, 0, "label", 6, 0, "=2+0");
        write_fixture_pair(&dir, &old, &new);
        write_scenario(
            &dir,
            "formula_shifted_origin",
            "regression",
            "Value range and formula range both start below row 1, and the \
             formula is not immediately below the label cell (a gap at row \
             6). General case of D-04's origin-translation fix, beyond the \
             row-1-vs-row-2 shape the `formula` fixture already covers \
             (RFC-036 #2, positive case).",
        );
        println!("✓ formula_shifted_origin");
    }

    // 10. Formula at the first cell — the D-04 negative control. Origins
    //     coincide; guards against a future fix that special-cases this
    //     instead of translating through absolute coordinates unconditionally.
    {
        let dir = base.join("formula_at_first_cell");
        let old = wb_with_formula(1, 0, "label", 0, 0, "=1+1"); // formula at row1, label at row2
        let new = wb_with_formula(1, 0, "label", 0, 0, "=2+0");
        write_fixture_pair(&dir, &old, &new);
        write_scenario(
            &dir,
            "formula_at_first_cell",
            "regression",
            "The formula cell IS the first populated cell — value-range and \
             formula-range origins coincide. Negative control for D-04: the \
             shape that would have hidden the defect entirely, and a guard \
             against a future fix that special-cases the coinciding-origin \
             case (RFC-036 #2, negative control).",
        );
        println!("✓ formula_at_first_cell");
    }

    // 11. RowSignature alignment — matched by whole-row content signature,
    //     not a key column. Zero test coverage before this scenario.
    {
        let dir = base.join("alignment_row_signature");
        let old = {
            let mut wb = new_workbook();
            let ws = wb.add_worksheet();
            for r in 0u32..5 {
                ws.write_string(r, 0, format!("row_{r}_a")).unwrap();
                ws.write_string(r, 1, format!("row_{r}_b")).unwrap();
            }
            wb.save_to_buffer().unwrap()
        };
        let new = {
            let mut wb = new_workbook();
            let ws = wb.add_worksheet();
            ws.write_string(0, 0, "inserted_a").unwrap();
            ws.write_string(0, 1, "inserted_b").unwrap();
            for r in 0u32..5 {
                ws.write_string(r + 1, 0, format!("row_{r}_a")).unwrap();
                ws.write_string(r + 1, 1, format!("row_{r}_b")).unwrap();
            }
            wb.save_to_buffer().unwrap()
        };
        write_fixture_pair(&dir, &old, &new);
        write_scenario(
            &dir,
            "alignment_row_signature",
            "regression",
            "Row inserted at the top of a 5-row sheet, matched by \
             RowSignature alignment (whole-row content signature, not a key \
             column) rather than RowKey. RowSignature had zero test \
             coverage before this scenario (RFC-036 #3).",
        );
        println!("✓ alignment_row_signature");
    }

    // 12. HeaderColumn alignment — header row plus data rows, one inserted.
    //     Zero test coverage before this scenario.
    {
        let dir = base.join("alignment_header_column");
        let old = {
            let mut wb = new_workbook();
            let ws = wb.add_worksheet();
            ws.write_string(0, 0, "id").unwrap();
            ws.write_string(0, 1, "value").unwrap();
            for r in 1u32..4 {
                ws.write_string(r, 0, format!("id_{r}")).unwrap();
                ws.write_string(r, 1, format!("val_{r}")).unwrap();
            }
            wb.save_to_buffer().unwrap()
        };
        let new = {
            let mut wb = new_workbook();
            let ws = wb.add_worksheet();
            ws.write_string(0, 0, "id").unwrap();
            ws.write_string(0, 1, "value").unwrap();
            ws.write_string(1, 0, "id_inserted").unwrap();
            ws.write_string(1, 1, "val_inserted").unwrap();
            for r in 1u32..4 {
                ws.write_string(r + 1, 0, format!("id_{r}")).unwrap();
                ws.write_string(r + 1, 1, format!("val_{r}")).unwrap();
            }
            wb.save_to_buffer().unwrap()
        };
        write_fixture_pair(&dir, &old, &new);
        write_scenario(
            &dir,
            "alignment_header_column",
            "regression",
            "Header row plus 3 data rows, one row inserted, matched by \
             HeaderColumn alignment. HeaderColumn had zero test coverage \
             before this scenario (RFC-036 #4).",
        );
        println!("✓ alignment_header_column");
    }

    // 13. Error values — one cell changes error kind, one stays the same
    //     error kind. CellError comparison had zero coverage at any level.
    {
        let dir = base.join("error_values");
        let old = {
            let mut wb = new_workbook();
            let ws = wb.add_worksheet();
            ws.write_formula(0, 0, Formula::new("1/0").set_result("#DIV/0!"))
                .unwrap();
            ws.write_formula(1, 0, Formula::new("A1").set_result("#N/A"))
                .unwrap();
            wb.save_to_buffer().unwrap()
        };
        let new = {
            let mut wb = new_workbook();
            let ws = wb.add_worksheet();
            ws.write_formula(0, 0, Formula::new("A2").set_result("#REF!"))
                .unwrap();
            ws.write_formula(1, 0, Formula::new("A1").set_result("#N/A"))
                .unwrap();
            wb.save_to_buffer().unwrap()
        };
        write_fixture_pair(&dir, &old, &new);
        write_scenario(
            &dir,
            "error_values",
            "regression",
            "One error cell changes kind (#DIV/0! -> #REF!); a second stays \
             the same error kind (#N/A unchanged). CellError comparison and \
             ValueDifferenceKind::ErrorKindChanged had zero test coverage \
             at any level before this scenario (RFC-036 #5).",
        );
        println!("✓ error_values");
    }

    // 14. Sheet reordered — two sheets swap position (SheetChange::Moved,
    //     never distinguished from Unchanged by any prior test); a third
    //     stays in place but its content changes.
    {
        let dir = base.join("sheet_reordered");
        let old = {
            let mut wb = new_workbook();
            let a = wb.add_worksheet();
            a.set_name("Alpha").unwrap();
            a.write_string(0, 0, "alpha_val").unwrap();
            let b = wb.add_worksheet();
            b.set_name("Beta").unwrap();
            b.write_string(0, 0, "beta_val").unwrap();
            let c = wb.add_worksheet();
            c.set_name("Gamma").unwrap();
            c.write_string(0, 0, "gamma_before").unwrap();
            wb.save_to_buffer().unwrap()
        };
        let new = {
            let mut wb = new_workbook();
            // Beta and Alpha swap order; Gamma stays in place but changes.
            let b = wb.add_worksheet();
            b.set_name("Beta").unwrap();
            b.write_string(0, 0, "beta_val").unwrap();
            let a = wb.add_worksheet();
            a.set_name("Alpha").unwrap();
            a.write_string(0, 0, "alpha_val").unwrap();
            let c = wb.add_worksheet();
            c.set_name("Gamma").unwrap();
            c.write_string(0, 0, "gamma_after").unwrap();
            wb.save_to_buffer().unwrap()
        };
        write_fixture_pair(&dir, &old, &new);
        write_scenario(
            &dir,
            "sheet_reordered",
            "regression",
            "Three sheets: Alpha and Beta swap position (same content, \
             different index — SheetChange::Moved), Gamma stays in place \
             but its one cell changes. SheetChange::Moved had never been \
             distinguished from Unchanged by any test before this scenario \
             (RFC-036 #6). Reordering needs no special API — sheets are \
             added in a different call order between old and new.",
        );
        println!("✓ sheet_reordered");
    }

    // 15. Date column — ordinary serial-based dates, one changed. No
    //     golden-corpus fixture used dates at all before this scenario,
    //     despite dates being where four M2 defects lived.
    {
        let dir = base.join("date_column");
        let date_format = Format::new().set_num_format("yyyy-mm-dd");
        let old = {
            let mut wb = new_workbook();
            let ws = wb.add_worksheet();
            ws.write_datetime_with_format(
                0,
                0,
                ExcelDateTime::from_ymd(2024, 1, 1).unwrap(),
                &date_format,
            )
            .unwrap();
            ws.write_datetime_with_format(
                1,
                0,
                ExcelDateTime::from_ymd(2024, 6, 15).unwrap(),
                &date_format,
            )
            .unwrap();
            wb.save_to_buffer().unwrap()
        };
        let new = {
            let mut wb = new_workbook();
            let ws = wb.add_worksheet();
            ws.write_datetime_with_format(
                0,
                0,
                ExcelDateTime::from_ymd(2024, 1, 1).unwrap(),
                &date_format,
            )
            .unwrap();
            ws.write_datetime_with_format(
                1,
                0,
                ExcelDateTime::from_ymd(2025, 6, 15).unwrap(),
                &date_format,
            )
            .unwrap();
            wb.save_to_buffer().unwrap()
        };
        write_fixture_pair(&dir, &old, &new);
        write_scenario(
            &dir,
            "date_column",
            "regression",
            "A column of ordinary serial-based dates, one changed. No \
             golden-corpus fixture used dates before this scenario, despite \
             dates being where four M2 defects lived (RFC-036 #7). Its \
             golden is blessed under the canonical `serde,chrono` feature \
             set, not `serde` alone — CellDateTime.iso is \
             chrono-feature-conditional, so the exact-JSON golden check is \
             gated on both features together (RFC-036 Handoff 02 correction \
             C-01). Also covered by the feature-invariant \
             date_column_fixture_detects_date_change in tests/integration.rs.",
        );
        println!("✓ date_column");
    }

    // 16. Non-ASCII text — sheet name and cell content. Zero coverage of
    //     XML-escaping / shared-string handling for non-ASCII content.
    {
        let dir = base.join("non_ascii_text");
        let old = wb_sheets(&[(
            "Café☕",
            &[(0, 0, "héllo wörld"), (1, 0, "unchanged 日本語")],
        )]);
        let new = wb_sheets(&[(
            "Café☕",
            &[(0, 0, "héllo wörld — updated"), (1, 0, "unchanged 日本語")],
        )]);
        write_fixture_pair(&dir, &old, &new);
        write_scenario(
            &dir,
            "non_ascii_text",
            "regression",
            "Non-ASCII sheet name and cell text (accented Latin, CJK, \
             emoji). XML-escaping and the shared-string table had zero \
             coverage of non-ASCII content before this scenario \
             (RFC-036 #8).",
        );
        println!("✓ non_ascii_text");
    }

    // 17. Chart sheet beside a worksheet — the chart-sheet coverage
    //     diagnostic had never fired in any test.
    {
        let dir = base.join("chart_sheet");
        let build_chart_workbook = |cell_value: &str| -> Vec<u8> {
            let mut wb = new_workbook();
            let ws = wb.add_worksheet();
            ws.write_number(0, 0, 1.0).unwrap();
            ws.write_number(1, 0, 2.0).unwrap();
            ws.write_string(3, 0, cell_value).unwrap();
            let mut chart = Chart::new(ChartType::Line);
            chart.add_series().set_values(("Sheet1", 0, 0, 1, 0));
            let cs = wb.add_chartsheet();
            cs.insert_chart(0, 0, &chart).unwrap();
            wb.save_to_buffer().unwrap()
        };
        let old = build_chart_workbook("before");
        let new = build_chart_workbook("after");
        write_fixture_pair(&dir, &old, &new);
        write_scenario(
            &dir,
            "chart_sheet",
            "regression",
            "A chart sheet beside an ordinary worksheet whose own cell \
             content changes. The chart-sheet coverage diagnostic had never \
             fired in any test before this scenario (RFC-036 #9).",
        );
        println!("✓ chart_sheet");
    }

    // -----------------------------------------------------------------------
    // RFC-036 §5.2 matrix — scenarios 10-11 (Handoff 03, XML-patched)
    // -----------------------------------------------------------------------

    // 18. Empty cell before content — a physically-present but empty <c>
    //     element precedes the sheet's real content. calamine filters
    //     DataRef::Empty out before computing range bounds, so this must
    //     NOT anchor the origin. rust_xlsxwriter has no reason to emit an
    //     empty cell element for a cell nothing was written to.
    {
        let dir = base.join("empty_cell_before_content");
        let build = |value: &str| -> Vec<u8> {
            let base_wb = wb_strings(&[(1, 0, value)]); // row 2 (0-based row 1), col A
            patch_xlsx_xml(&base_wb, "xl/worksheets/sheet1.xml", |xml| {
                xml.replacen(
                    "<sheetData>",
                    "<sheetData><row r=\"1\"><c r=\"A1\"/></row>",
                    1,
                )
            })
        };
        let old = build("before");
        let new = build("after");
        write_fixture_pair(&dir, &old, &new);
        write_scenario(
            &dir,
            "empty_cell_before_content",
            "regression",
            "A physically-present but empty cell (<c r=\"A1\"/>, no value or \
             type) precedes the sheet's real content at A2. calamine \
             filters DataRef::Empty cells out before computing the range's \
             start/end, so this must not anchor the origin at A1 — a real \
             and non-obvious behaviour with no prior test (RFC-036 #10). \
             Generated then XML-patched: rust_xlsxwriter has no reason to \
             emit an empty cell element for a cell nothing was written to.",
        );
        println!("✓ empty_cell_before_content");
    }

    // 19. ISO datetime — a t="d" cell, calamine's DateTimeIso path.
    //     rust_xlsxwriter cannot emit this cell type. Promotes RFC-035
    //     Handoff 05's hand-built reachability test into a durable
    //     corpus trip-wire for the exact bug D-01 was.
    {
        let dir = base.join("iso_datetime");
        let build = |iso: &str| -> Vec<u8> {
            let base_wb = wb_strings(&[(0, 0, "label")]);
            patch_xlsx_xml(&base_wb, "xl/worksheets/sheet1.xml", |xml| {
                xml.replacen(
                    "</sheetData>",
                    &format!("<row r=\"2\"><c r=\"A2\" t=\"d\"><v>{iso}</v></c></row></sheetData>"),
                    1,
                )
            })
        };
        let old = build("2024-01-01T00:00:00");
        let new = build("2099-12-31T23:59:59");
        write_fixture_pair(&dir, &old, &new);
        write_scenario(
            &dir,
            "iso_datetime",
            "regression",
            "An ISO-typed date/time cell (t=\"d\", calamine's DateTimeIso \
             path). rust_xlsxwriter cannot emit this cell type; generated \
             then XML-patched. Promotes RFC-035 Handoff 05's hand-built \
             d01_iso_datetime_reachability_end_to_end test into a durable \
             corpus trip-wire for the exact bug D-01 was (RFC-036 #11).",
        );
        println!("✓ iso_datetime");
    }

    println!("\nAll fixtures generated in {}", base.display());
    println!("expected.json is not written here — bless goldens with:");
    println!("  BLESS=1 cargo test --features serde,chrono -- generated_fixtures_match_golden");
}
