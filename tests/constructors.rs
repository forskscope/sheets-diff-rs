// RFC-002, RFC-004, RFC-006: constructor and error-model tests.

use sheets_diff::core::diff::Diff;
use sheets_diff::core::error::{SheetsDiffError, WorkbookSide};
use sheets_diff::core::unified_format::unified_diff;

const OLD_FIXTURE: &str = "tests/fixtures/file1.xlsx";
const NEW_FIXTURE: &str = "tests/fixtures/file2.xlsx";

// ---------------------------------------------------------------------------
// Existing golden fixture test (preserved from v1.1.4)
// ---------------------------------------------------------------------------

#[test]
fn golden_fixture_output_is_stable() {
    // v1.2.0: the spurious D11 empty-formula entry from v1.1.4 is gone.
    // Previously the old code compared Data::Empty vs Data::String(""), which
    // emitted a diff for a cell that had no formula in the old workbook and an
    // empty formula string in the new workbook. The new code treats both as
    // equivalent empty values, which is the correct behavior.
    const EXPECT: &str = r#"--- tests/fixtures/file1.xlsx (sheet names)
+++ tests/fixtures/file2.xlsx (sheet names)
- Sheet1_2
+ Sheetzz
--- tests/fixtures/file1.xlsx [Sheet1]
+++ tests/fixtures/file2.xlsx [Sheet1]
@@ A1(1,1) value @@
- 1
@@ B2(2,2) value @@
- 2
+ 今日は世界
@@ B4(4,2) value @@
+ a
@@ C6(6,3) value @@
+ hej
@@ D10(10,4) value @@
- 2
+ 8
@@ D10(10,4) formula @@
- 1+1
+ 2*4
@@ D12(12,4) value @@
+ a123
@@ D12(12,4) formula @@
+ "a"&123
@@ W55(55,23) value @@
+ っｓ
"#;

    let diff = Diff::new(OLD_FIXTURE, NEW_FIXTURE);
    let output = unified_diff(&diff).format();
    assert_eq!(format!("{output}"), EXPECT);
}

// ---------------------------------------------------------------------------
// Fallible path constructor — RFC-002
// ---------------------------------------------------------------------------

#[test]
fn try_new_succeeds_on_valid_fixtures() {
    let diff = Diff::try_new(OLD_FIXTURE, NEW_FIXTURE);
    assert!(diff.is_ok(), "expected Ok, got: {diff:?}");
}

#[test]
fn try_new_missing_old_returns_open_workbook_old() {
    let result = Diff::try_new("missing-old.xlsx", NEW_FIXTURE);
    assert!(
        matches!(
            result,
            Err(SheetsDiffError::OpenWorkbook {
                side: WorkbookSide::Old,
                ..
            })
        ),
        "unexpected result: {result:?}"
    );
}

#[test]
fn try_new_missing_new_returns_open_workbook_new() {
    let result = Diff::try_new(OLD_FIXTURE, "missing-new.xlsx");
    assert!(
        matches!(
            result,
            Err(SheetsDiffError::OpenWorkbook {
                side: WorkbookSide::New,
                ..
            })
        ),
        "unexpected result: {result:?}"
    );
}

#[test]
fn try_new_non_xlsx_file_returns_error_not_panic() {
    let result = Diff::try_new("tests/fixtures/non-xlsx.txt", NEW_FIXTURE);
    assert!(
        result.is_err(),
        "expected Err for non-xlsx input, got Ok"
    );
}

/// `Diff::new` must still panic on invalid input (v1.1.4 contract).
#[test]
#[should_panic]
fn diff_new_panics_on_missing_files() {
    let _ = Diff::new("no-such-old.xlsx", "no-such-new.xlsx");
}

/// Error types implement Display and Error.
#[test]
fn error_display_is_human_readable() {
    let result = Diff::try_new("tests/fixtures/non-xlsx.txt", NEW_FIXTURE);
    let err = result.unwrap_err();
    let msg = err.to_string();
    assert!(!msg.is_empty(), "error Display must not be empty");
    // The standard Error trait source chain must not loop.
    let _ = std::error::Error::source(&err);
}

// ---------------------------------------------------------------------------
// Reader-based constructor — RFC-004
// ---------------------------------------------------------------------------

#[test]
fn try_from_named_readers_succeeds_on_fixture_bytes() {
    use std::io::Cursor;

    let old_bytes = std::fs::read(OLD_FIXTURE).expect("read old fixture");
    let new_bytes = std::fs::read(NEW_FIXTURE).expect("read new fixture");

    let diff = Diff::try_from_named_readers(
        "old.xlsx",
        Cursor::new(old_bytes),
        "new.xlsx",
        Cursor::new(new_bytes),
    )
    .expect("reader diff should succeed");

    assert_eq!(diff.old_filepath, "old.xlsx");
    assert_eq!(diff.new_filepath, "new.xlsx");
}

#[test]
fn reader_output_matches_path_output() {
    use std::io::Cursor;

    let path_diff = Diff::try_new(OLD_FIXTURE, NEW_FIXTURE).unwrap();

    let old_bytes = std::fs::read(OLD_FIXTURE).expect("read old fixture");
    let new_bytes = std::fs::read(NEW_FIXTURE).expect("read new fixture");
    let reader_diff = Diff::try_from_named_readers(
        OLD_FIXTURE,
        Cursor::new(old_bytes),
        NEW_FIXTURE,
        Cursor::new(new_bytes),
    )
    .unwrap();

    // Both diffs must agree on cell-level content.
    assert_eq!(
        path_diff.cell_diffs.len(),
        reader_diff.cell_diffs.len(),
        "cell_diffs length mismatch"
    );
    assert_eq!(
        path_diff.sheet_diff.len(),
        reader_diff.sheet_diff.len(),
        "sheet_diff length mismatch"
    );
}

#[test]
fn reader_constructor_returns_open_reader_error_for_non_xlsx() {
    use std::io::Cursor;
    let junk = Cursor::new(b"not an xlsx".to_vec());
    let new_bytes = std::fs::read(NEW_FIXTURE).expect("read new fixture");

    let result = Diff::try_from_named_readers("junk", junk, "new.xlsx", Cursor::new(new_bytes));
    assert!(
        matches!(
            result,
            Err(SheetsDiffError::OpenReader {
                side: WorkbookSide::Old,
                ..
            })
        ),
        "unexpected result: {result:?}"
    );
}

// ---------------------------------------------------------------------------
// Stdlib compatibility — Diff::diff() clone helper (v1.1.4 API)
// ---------------------------------------------------------------------------

#[test]
fn diff_clone_helper_compiles_and_equals() {
    let mut d = Diff::try_new(OLD_FIXTURE, NEW_FIXTURE).unwrap();
    let cloned = d.diff();
    assert_eq!(d.old_filepath, cloned.old_filepath);
    assert_eq!(d.new_filepath, cloned.new_filepath);
}
