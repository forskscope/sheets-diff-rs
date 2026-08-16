#![allow(dead_code)]
//! Shared fixture-generation helpers for integration tests (RFC-015).
//!
//! All helpers return `Vec<u8>` so tests stay I/O-free; `compare_bytes` is
//! the preferred entry point for fixture-driven tests.
//!
//! `examples/gen-fixtures.rs` duplicates a subset of the builders below
//! rather than sharing this module (an example cannot depend on `tests/`).
//! That copy deliberately pins a fixed document-creation timestamp on every
//! workbook it builds — the builders here do not, and must not, because
//! these back ad-hoc in-memory comparisons with no byte-reproducibility
//! requirement, while the example's output is the committed fixture corpus.
//! If you change a builder signature here that has a counterpart there,
//! check whether the other needs the same change.

use rust_xlsxwriter::{Formula, Workbook};

// ---------------------------------------------------------------------------
// Basic builders
// ---------------------------------------------------------------------------

/// Single-sheet workbook with string cells. `cells` is `(0-based row, 0-based col, value)`.
pub fn wb_strings(cells: &[(u32, u16, &str)]) -> Vec<u8> {
    let mut wb = Workbook::new();
    let ws = wb.add_worksheet();
    for (row, col, val) in cells {
        ws.write_string(*row, *col, *val).unwrap();
    }
    wb.save_to_buffer().unwrap()
}

/// Single-sheet workbook with numeric (float) cells.
pub fn wb_numbers(cells: &[(u32, u16, f64)]) -> Vec<u8> {
    let mut wb = Workbook::new();
    let ws = wb.add_worksheet();
    for (row, col, val) in cells {
        ws.write_number(*row, *col, *val).unwrap();
    }
    wb.save_to_buffer().unwrap()
}

/// Single-sheet workbook with boolean cells.
pub fn wb_bools(cells: &[(u32, u16, bool)]) -> Vec<u8> {
    let mut wb = Workbook::new();
    let ws = wb.add_worksheet();
    for (row, col, val) in cells {
        ws.write_boolean(*row, *col, *val).unwrap();
    }
    wb.save_to_buffer().unwrap()
}

/// Workbook with one sheet containing both a string and a formula cell.
pub fn wb_with_formula(
    value_row: u32,
    value_col: u16,
    value: &str,
    formula_row: u32,
    formula_col: u16,
    formula: &str,
) -> Vec<u8> {
    let mut wb = Workbook::new();
    let ws = wb.add_worksheet();
    ws.write_string(value_row, value_col, value).unwrap();
    ws.write_formula(formula_row, formula_col, Formula::new(formula))
        .unwrap();
    wb.save_to_buffer().unwrap()
}

/// Empty workbook (one sheet, no cells).
pub fn wb_empty() -> Vec<u8> {
    let mut wb = Workbook::new();
    wb.add_worksheet();
    wb.save_to_buffer().unwrap()
}

/// One named sheet's cells, as `(name, cells)`.
pub type SheetSpec<'a> = (&'a str, &'a [(u32, u16, &'a str)]);

/// Workbook with multiple named sheets, each optionally populated.
pub fn wb_sheets(sheets: &[SheetSpec]) -> Vec<u8> {
    let mut wb = Workbook::new();
    for (name, cells) in sheets {
        let ws = wb.add_worksheet();
        ws.set_name(*name).unwrap();
        for (row, col, val) in *cells {
            ws.write_string(*row, *col, *val).unwrap();
        }
    }
    wb.save_to_buffer().unwrap()
}

/// Workbook with a cell at a high column index (tests A1 encoding).
pub fn wb_wide_column(row: u32, col: u16, value: &str) -> Vec<u8> {
    let mut wb = Workbook::new();
    let ws = wb.add_worksheet();
    ws.write_string(row, col, value).unwrap();
    wb.save_to_buffer().unwrap()
}

/// Workbook with sparse data: only a few cells far apart.
pub fn wb_sparse(cells: &[(u32, u16, &str)]) -> Vec<u8> {
    wb_strings(cells)
}

/// Large generated workbook (many rows). Marked `#[ignore]` in tests.
pub fn wb_large(rows: u32, cols: u16, prefix: &str) -> Vec<u8> {
    let mut wb = Workbook::new();
    let ws = wb.add_worksheet();
    for r in 0..rows {
        for c in 0..cols {
            ws.write_string(r, c, format!("{prefix}_{r}_{c}")).unwrap();
        }
    }
    wb.save_to_buffer().unwrap()
}

// ---------------------------------------------------------------------------
// Raw XML patching (RFC-035 Handoff 05 — D-01/D-02 reachability)
// ---------------------------------------------------------------------------

/// Rewrite one XML entry inside a generated `.xlsx` (itself just a ZIP
/// archive) with `patch` applied to its UTF-8 text content. Every other
/// entry is copied through unchanged.
///
/// Exists to reach calamine code paths `rust_xlsxwriter`'s public API
/// cannot produce directly — e.g. a raw `t="d"` ISO-typed cell (D-01), or
/// `<workbookPr date1904="1"/>` (D-02). Panics if `entry_path` is not found,
/// since a silently-no-op patch would make a test that "passes" prove
/// nothing.
pub fn patch_xlsx_xml(
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
