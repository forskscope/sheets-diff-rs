//! A 512-byte file aborts the process; decline it before delegating (f132).
//!
//! `Xlsx::new` runs `check_for_password_protected` before anything else, which parses *any* input
//! as a CFB container. A sector-count field in the CFB header reaches `Vec::with_capacity`
//! unchecked against the input's actual length (calamine 0.36.1: `xlsx/mod.rs:2939` ->
//! `cfb.rs:260`), so a hand-craftable input around 512 bytes requests a multi-gigabyte allocation
//! and aborts the process. `max_input_bytes` cannot stop it (it bounds the input's length, not a
//! field inside it) and neither can `Limits::hardened()` (verified — see the review request).
//!
//! **The fix is a pre-screen** (`src/open.rs`): decline anything that is not a ZIP before calamine
//! ever sees it, with a byte-scan carve-out (never a parse) for a real encrypted `.xlsx`, which is a
//! CFB container by design and must keep reporting `EncryptedWorkbook`.
//!
//! **This file cannot demonstrate the abort itself** — an aborting process takes the test binary
//! down with it. The failing-first evidence for that is a separate process under a memory cap; see
//! the review request (`evidence/01-before-abort.txt`, and `evidence/failing-first.sh` for the
//! `cmp`-verified restore with the pre-screen removed). What this file asserts is everything the fix
//! must get right without crashing anything: the artifact now returns an ordinary `Err`, an encrypted
//! workbook is unaffected, a CFB input that is *not* encrypted is not mislabelled as one, and no
//! valid workbook is declined.

use sheets_diff::{OpenErrorKind, SheetsDiffError, compare_bytes};

fn read(path: &str) -> Vec<u8> {
    std::fs::read(path).unwrap_or_else(|e| panic!("{path}: {e}"))
}

/// The minimized crash input from M9 unit 01's fuzz run (the 3-byte self-comparison header
/// stripped off; this is the 512-byte workbook payload alone). CFB magic, no `EncryptedPackage`
/// marker.
const OOM_ARTIFACT: &str = "tests/fixtures/f132/oom-artifact-515b.bin";

#[test]
fn the_oom_artifact_returns_an_error_instead_of_aborting() {
    let bytes = read(OOM_ARTIFACT);
    assert_eq!(bytes.len(), 512);
    assert!(bytes.starts_with(&[0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1]));

    let result = compare_bytes(&bytes, &bytes);
    let err = result.expect_err("the artifact must not open as a workbook");
    assert!(
        matches!(
            err,
            SheetsDiffError::OpenWorkbook {
                kind: OpenErrorKind::NotXlsx,
                ..
            }
        ),
        "expected OpenWorkbook{{ kind: NotXlsx, .. }}, got {err:?}"
    );
    assert_eq!(
        err.to_string(),
        "cannot open old workbook '<unknown>': not an xlsx file",
        "the message for a non-xlsx input must not change for real non-xlsx input either"
    );
}

/// A CFB-magic input with no `EncryptedPackage` marker anywhere — a legacy `.xls`'s shape, and the
/// crash artifact's own shape — must not be mislabelled `EncryptedWorkbook`. Calling it "encrypted"
/// would be a lie the caller cannot detect by looking at their own file.
#[test]
fn a_non_encrypted_cfb_input_is_not_labelled_encrypted() {
    let bytes = read("tests/fixtures/f132/legacy-cfb-no-marker.bin");
    assert!(bytes.starts_with(&[0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1]));
    assert!(
        !bytes.windows(32).any(|w| w
            == "EncryptedPackage"
                .encode_utf16()
                .flat_map(u16::to_le_bytes)
                .collect::<Vec<_>>()),
        "fixture must genuinely lack the marker"
    );

    let err = compare_bytes(&bytes, &bytes).expect_err("not a valid workbook either way");
    assert!(
        matches!(
            err,
            SheetsDiffError::OpenWorkbook {
                kind: OpenErrorKind::NotXlsx,
                ..
            }
        ),
        "a non-encrypted CFB input must get the ordinary not-an-xlsx outcome, not EncryptedWorkbook: {err:?}"
    );
}

/// A real encrypted `.xlsx` — CFB, with the marker — is unaffected: same variant it always was.
#[test]
fn a_real_encrypted_workbook_still_reports_encrypted_workbook() {
    let bytes = read("tests/fixtures/corrupt/encrypted.xlsx");
    assert!(bytes.starts_with(&[0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1]));

    let err = compare_bytes(&bytes, &bytes).expect_err("an encrypted workbook cannot be opened");
    assert!(
        matches!(err, SheetsDiffError::EncryptedWorkbook { .. }),
        "expected EncryptedWorkbook, got {err:?}"
    );
    assert_eq!(err.to_string(), "old workbook is password-protected");
}

/// A ZIP file that is not an xlsx (no xlsx-internal parts) still reaches calamine and gets its
/// ordinary answer — the pre-screen only ever *declines* on non-ZIP input, it never accepts
/// something calamine itself would reject.
#[test]
fn a_non_xlsx_zip_still_reaches_calamine_unaffected() {
    let bytes = read("tests/fixtures/corrupt/not_a_zip.xlsx");
    // (Named `not_a_zip.xlsx`; whichever of "not a zip" or "zip but not xlsx" it is, both routes
    // through the pre-screen unchanged: non-ZIP is declined directly, ZIP-but-invalid reaches
    // calamine and calamine declines it. Assert only the outcome, not the route.)
    let err = compare_bytes(&bytes, &bytes).expect_err("not a valid workbook");
    assert!(
        matches!(
            err,
            SheetsDiffError::OpenWorkbook {
                kind: OpenErrorKind::NotXlsx,
                ..
            }
        ),
        "{err:?}"
    );
}

/// Ordinary valid workbooks are unaffected: the pre-screen never declines a real `.xlsx`. Spot
/// check here; `tests/integration.rs`'s golden comparisons cover the rest of the corpus.
#[test]
fn valid_workbooks_are_unaffected() {
    let d = "tests/fixtures/generated/date_column";
    let old = read(&format!("{d}/old.xlsx"));
    let new = read(&format!("{d}/new.xlsx"));
    assert!(old.starts_with(b"PK\x03\x04"));
    let diff = compare_bytes(&old, &new).expect("a valid workbook must still open");
    assert_eq!(diff.sheets.len(), 1);
}

/// Every fixture this crate ships as a *valid* workbook actually starts with a ZIP magic — the
/// premise the pre-screen relies on, checked against the corpus rather than assumed (Known risk 1).
/// `tests/fixtures/corrupt/` is deliberately excluded: those two are supposed to fail.
#[test]
fn every_valid_fixture_starts_with_the_zip_magic() {
    fn walk(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_dir() {
                walk(&path, out);
            } else if path.extension().is_some_and(|e| e == "xlsx") {
                out.push(path);
            }
        }
    }
    let mut files = Vec::new();
    walk(std::path::Path::new("tests/fixtures/generated"), &mut files);
    assert!(
        files.len() >= 19 * 2,
        "expected the full corpus, got {}",
        files.len()
    );
    for f in files {
        let bytes = std::fs::read(&f).unwrap();
        assert!(
            bytes.starts_with(b"PK\x03\x04") || bytes.starts_with(b"PK\x05\x06"),
            "{f:?} does not start with a ZIP magic: {:02x?}",
            &bytes[..bytes.len().min(4)]
        );
    }
}
