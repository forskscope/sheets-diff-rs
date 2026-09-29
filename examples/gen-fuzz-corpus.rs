//! Fuzz corpus generator (M9 unit 01).
//!
//! Run with:
//!   cargo run --example gen-fuzz-corpus
//!
//! Not a test: `cargo test` never runs it, so a checkout only changes the corpus when this is
//! invoked explicitly (`tests/fuzz_corpus_reaches_reader.rs` checks what is *committed*, not that
//! this generator was just run). Every generated workbook pins a fixed creation timestamp, the same
//! way `examples/gen-fixtures.rs` does, so re-running this generator reproduces the committed corpus
//! byte-for-byte — "reproducible" per the handoff, not "a set of opaque blobs": the corpus files are
//! still binary, necessarily, but this script regenerates the same bytes from the same inputs.
//!
//! **Framing.** Seeds for `fuzz_open_xlsx_bytes` and `fuzz_self_comparison` are built with the exact
//! functions those targets (and `tests/fuzz_corpus_reaches_reader.rs`) use, via `include!` of
//! `fuzz/src/framing.rs` and `fuzz/src/self_seed.rs` — three files agreeing by construction rather
//! than by three copies staying in sync.
//!
//! **What each seed is derived from**, and why: see the review request,
//! `.git-exclude/review-request/m9-01-corpus-reaches-the-reader/README.md` §3 (RFC-028 §6 category
//! table) and §4 (the self-comparison seeds, including the one f130's original defect fails on).

use std::path::Path;

use rust_xlsxwriter::{DocProperties, ExcelDateTime, Workbook};

include!("../fuzz/src/framing.rs");
include!("../fuzz/src/self_seed.rs");

// ---------------------------------------------------------------------------
// Fixed-timestamp workbook construction (matches examples/gen-fixtures.rs; see its own comment on
// why this is not shared with tests/support.rs's unpinned builders)
// ---------------------------------------------------------------------------

fn new_workbook() -> Workbook {
    let mut wb = Workbook::new();
    let date = ExcelDateTime::from_ymd(2020, 1, 1).expect("valid fixed fixture date");
    wb.set_properties(&DocProperties::new().set_creation_datetime(&date));
    wb
}

/// RFC-028 §6 category "workbook with many sheets": more sheets than any existing fixture has, kept
/// small (one cell each) so the file stays a few KB.
fn many_sheets(n: u32) -> Vec<u8> {
    let mut wb = new_workbook();
    for i in 0..n {
        let ws = wb.add_worksheet();
        ws.set_name(format!("S{i}")).unwrap();
        ws.write_string(0, 0, format!("sheet {i}")).unwrap();
    }
    wb.save_to_buffer().unwrap()
}

/// The shape the self-comparison oracle is meant to catch (RFC-028's own report, and this unit's
/// acceptance criterion 3): `rows` rows, a unique id in column A except every 20th row, which has
/// none. `RowKey` on column 1 pairs the identical keyless rows with each other (f130); with that
/// pairing removed, they are dropped before comparison and a self-comparison stops being empty.
fn blank_key_rows(rows: u32) -> Vec<u8> {
    let mut wb = new_workbook();
    let ws = wb.add_worksheet();
    for r in 0..rows {
        if r % 20 != 19 {
            ws.write_string(r, 0, format!("id{r:04}")).unwrap();
        }
        ws.write_string(r, 1, format!("v{r}")).unwrap();
    }
    wb.save_to_buffer().unwrap()
}

/// RFC-028 §6 category "valid ZIP but not XLSX": an archive with none of the parts an xlsx needs
/// (`[Content_Types].xml`, `xl/workbook.xml`, ...), so it opens as a ZIP and fails as a workbook.
/// Built with the same `zip` crate version the main crate pins as a dev-dependency (see
/// `Cargo.toml`'s comment on why), so this generator adds no new dependency to the tree.
fn valid_zip_not_xlsx() -> Vec<u8> {
    let mut buf = Vec::new();
    {
        let mut z = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
        let opts: zip::write::FileOptions<()> = zip::write::FileOptions::default();
        z.start_file("hello.txt", opts).unwrap();
        std::io::Write::write_all(&mut z, b"not a workbook").unwrap();
        z.finish().unwrap();
    }
    buf
}

fn read(path: &str) -> Vec<u8> {
    std::fs::read(Path::new(path)).unwrap_or_else(|e| panic!("{path}: {e}"))
}

fn write(dir: &str, name: &str, bytes: &[u8]) {
    let d = Path::new("fuzz/corpus").join(dir);
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(d.join(name), bytes).unwrap();
    println!("wrote fuzz/corpus/{dir}/{name} ({} bytes)", bytes.len());
}

fn fixture(scenario: &str, side: &str) -> Vec<u8> {
    read(&format!("tests/fixtures/generated/{scenario}/{side}.xlsx"))
}

fn main() {
    // ---- Generated (non-fixture) workbooks, reused across both corpora ----
    let many = many_sheets(24);
    let blank_keys = blank_key_rows(400);
    let not_xlsx = valid_zip_not_xlsx();
    let encrypted = read("tests/fixtures/corrupt/encrypted.xlsx");
    let date_old = fixture("date_column", "old");
    let truncated_real = date_old[..date_old.len() / 2].to_vec();

    // ---- fuzz_open_xlsx_bytes: paired seeds. The three hand-written seeds (empty, random_bytes,
    // truncated_zip) are untouched — see tests/fuzz_corpus_reaches_reader.rs and the review request
    // for why they stay. RFC-028 §6 category named per seed. ----
    let paired: &[(&str, Vec<u8>, Vec<u8>)] = &[
        (
            "paired_date_column",
            date_old.clone(),
            fixture("date_column", "new"),
        ), // typed content
        (
            "paired_formula",
            fixture("formula", "old"),
            fixture("formula", "new"),
        ), // formulas
        (
            "paired_wide_columns",
            fixture("wide_columns", "old"),
            fixture("wide_columns", "new"),
        ), // wide columns
        (
            "paired_chart_sheet",
            fixture("chart_sheet", "old"),
            fixture("chart_sheet", "new"),
        ), // unsupported objects
        (
            "paired_empty_sheet",
            fixture("empty_sheet", "old"),
            fixture("empty_sheet", "new"),
        ), // empty sheets
        ("paired_many_sheets", many.clone(), many.clone()), // many sheets
        ("paired_encrypted", encrypted.clone(), encrypted.clone()), // password-protected
        (
            "paired_valid_zip_not_xlsx",
            not_xlsx.clone(),
            not_xlsx.clone(),
        ), // valid ZIP, not XLSX
        (
            "paired_truncated_real_xlsx",
            truncated_real.clone(),
            date_old.clone(),
        ), // truncated XLSX
    ];
    for (name, old, new) in paired {
        write("fuzz_open_xlsx_bytes", name, &make_seed(old, new));
    }

    // ---- fuzz_self_comparison: header + one workbook. Mode bytes: 0 = Positional, 1 = RowKey
    // (column = 1 + b(1) % 5), 2 = RowSignature; see self_seed.rs. ----
    let self_seeds: &[(&str, [u8; SELF_SEED_HEADER_LEN], Vec<u8>)] = &[
        ("self_date_column", [0, 0, 0], fixture("date_column", "old")),
        (
            "self_formula_rowsignature",
            [2, 0, 0],
            fixture("formula", "old"),
        ),
        (
            "self_wide_columns_rowkey",
            [1, 2, 0],
            fixture("wide_columns", "old"),
        ),
        ("self_chart_sheet", [0, 0, 0], fixture("chart_sheet", "old")),
        ("self_empty_sheet", [0, 0, 0], fixture("empty_sheet", "old")),
        ("self_many_sheets_rowsignature", [2, 0, 0], many.clone()),
        ("self_encrypted", [0, 0, 0], encrypted.clone()),
        ("self_valid_zip_not_xlsx", [0, 0, 0], not_xlsx.clone()),
        (
            "self_truncated_real_xlsx",
            [0, 0, 0],
            truncated_real.clone(),
        ),
        // The regression seed for acceptance criterion 3: RowKey on the id column, over the shape
        // that reproduces f130. Positional and RowSignature controls alongside it, per the handoff
        // ("do not expect Positional or RowSignature to move").
        ("self_blank_key_rows_rowkey", [1, 0, 0], blank_keys.clone()),
        (
            "self_blank_key_rows_positional",
            [0, 0, 0],
            blank_keys.clone(),
        ),
        (
            "self_blank_key_rows_rowsignature",
            [2, 0, 0],
            blank_keys.clone(),
        ),
    ];
    for (name, header, workbook) in self_seeds {
        write(
            "fuzz_self_comparison",
            name,
            &make_self_seed(*header, workbook),
        );
    }
}
