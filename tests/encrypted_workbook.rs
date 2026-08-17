//! Encrypted-workbook detection (RFC-032, M5 Handoff 03).
//!
//! `SheetsDiffError::EncryptedWorkbook` has existed since v2.0.0, has a
//! dedicated CLI exit code (M4 unit 03), and had never been tested --
//! `tests/fixtures/corrupt/encrypted.xlsx` (see that directory's README for
//! what it actually is: a CFB container shaped to trigger the detection,
//! not a real encrypted workbook) closes that gap.

use sheets_diff::{SheetsDiffError, Side};

fn encrypted_bytes() -> Vec<u8> {
    std::fs::read("tests/fixtures/corrupt/encrypted.xlsx").unwrap()
}

fn wb_empty() -> Vec<u8> {
    // A trivially valid, empty workbook -- built inline rather than pulled
    // from tests/support.rs, since this file's only other dependency is the
    // committed fixture above and adding a `mod support;` for one helper
    // would be a heavier coupling than duplicating four lines.
    let mut wb = rust_xlsxwriter::Workbook::new();
    wb.add_worksheet();
    wb.save_to_buffer().unwrap()
}

#[test]
fn encrypted_bytes_are_not_accidentally_a_valid_xlsx() {
    // Sanity check on the fixture itself, not the detection: if this ever
    // fails, `encrypted.xlsx` has been replaced by something that opens
    // successfully, and every test below would be vacuous.
    assert!(
        calamine::open_workbook::<calamine::Xlsx<_>, _>("tests/fixtures/corrupt/encrypted.xlsx")
            .is_err()
    );
}

#[test]
fn compare_bytes_reports_encrypted_workbook_old_side() {
    let result = sheets_diff::compare_bytes(encrypted_bytes(), wb_empty());
    assert!(matches!(
        result,
        Err(SheetsDiffError::EncryptedWorkbook { side: Side::Old })
    ));
}

#[test]
fn compare_bytes_reports_encrypted_workbook_new_side() {
    let result = sheets_diff::compare_bytes(wb_empty(), encrypted_bytes());
    assert!(matches!(
        result,
        Err(SheetsDiffError::EncryptedWorkbook { side: Side::New })
    ));
}

#[test]
fn compare_paths_reports_encrypted_workbook() {
    let result = sheets_diff::compare_paths(
        "tests/fixtures/corrupt/encrypted.xlsx",
        "tests/fixtures/corrupt/encrypted.xlsx",
    );
    assert!(matches!(
        result,
        Err(SheetsDiffError::EncryptedWorkbook { side: Side::Old })
    ));
}

#[test]
fn rendered_message_names_the_condition() {
    let old_err = sheets_diff::compare_bytes(encrypted_bytes(), wb_empty()).unwrap_err();
    let old_rendered = old_err.to_string();
    assert!(
        old_rendered.contains("password-protected"),
        "expected the message to name the condition, got: {old_rendered}"
    );
    assert!(
        old_rendered.contains("old"),
        "expected the message to name which side, got: {old_rendered}"
    );

    let new_err = sheets_diff::compare_bytes(wb_empty(), encrypted_bytes()).unwrap_err();
    let new_rendered = new_err.to_string();
    assert!(new_rendered.contains("password-protected"));
    assert!(new_rendered.contains("new"));

    // Negative control: an ordinary open failure must NOT produce this
    // message -- otherwise the assertions above would pass for the wrong
    // reason (any error message, not specifically this one).
    let corrupt_err = sheets_diff::compare_bytes(
        std::fs::read("tests/fixtures/corrupt/not_a_zip.xlsx").unwrap(),
        wb_empty(),
    )
    .unwrap_err();
    assert!(!corrupt_err.to_string().contains("password-protected"));
}
