//! Source-path privacy (RFC-016, M5 Handoff 02).
//!
//! `lib.rs` promises, in prose, that a caller's filesystem path never
//! survives into a diff result beyond its file name, and that a non-UTF-8
//! path is fully supported rather than causing a panic. Nothing tested
//! either claim before this file. Every test here asserts the positive
//! shape (the value the property implies), not only an absence -- a test
//! that only checks a substring is missing would pass trivially if the
//! field were `None` for an unrelated reason.

mod support;
use support::wb_empty;

use sheets_diff::{compare_bytes, compare_paths, compare_readers};

/// A unique, human-identifiable directory name a test asserts never
/// survives -- distinctive enough that an accidental substring match
/// elsewhere in a rendered result would be a real finding, not noise.
const SENSITIVE_DIR: &str = "clients_confidential_do_not_leak";

fn unique_root(tag: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "sheets-diff-privacy-{tag}-{}-{}",
        std::process::id(),
        tag
    ))
}

// ---------------------------------------------------------------------------
// 1 & 2: the parent directory does not survive, anywhere in the result
// ---------------------------------------------------------------------------

#[test]
fn parent_directory_does_not_survive_into_the_result() {
    let root = unique_root("dir");
    let dir = root.join(SENSITIVE_DIR);
    std::fs::create_dir_all(&dir).unwrap();
    let old_path = dir.join("old.xlsx");
    let new_path = dir.join("new.xlsx");
    std::fs::write(&old_path, wb_empty()).unwrap();
    std::fs::write(&new_path, wb_empty()).unwrap();

    let diff = compare_paths(&old_path, &new_path).unwrap();

    // Positive shape: display_name IS the file name, not merely "not the
    // directory" and not merely present.
    assert_eq!(diff.old.source.display_name.as_deref(), Some("old.xlsx"));
    assert_eq!(diff.new.source.display_name.as_deref(), Some("new.xlsx"));

    // Nowhere in the whole result, not just the field the design targets:
    // Debug-format the entire WorkbookDiff and check the directory is
    // absent from it, plus both text renderers actually shipped.
    let debug_repr = format!("{diff:?}");
    let summary = sheets_diff::output::text::render_summary(&diff);
    let unified = sheets_diff::output::text::render_unified(&diff);

    for (label, rendered) in [
        ("Debug", debug_repr.as_str()),
        ("render_summary", summary.as_str()),
        ("render_unified", unified.as_str()),
    ] {
        assert!(
            !rendered.contains(SENSITIVE_DIR),
            "{label} output leaked the parent directory: {rendered}"
        );
        assert!(
            !rendered.contains(root.to_string_lossy().as_ref()),
            "{label} output leaked the full temp root: {rendered}"
        );
    }
    // The file names ARE expected to appear in the renderers (that's the
    // documented, intended behaviour) -- confirms the absence above isn't
    // because the renderers omit source names entirely.
    assert!(summary.contains("old.xlsx") && summary.contains("new.xlsx"));
    assert!(unified.contains("old.xlsx") && unified.contains("new.xlsx"));

    std::fs::remove_dir_all(&root).ok();
}

// ---------------------------------------------------------------------------
// 3: non-UTF-8 file name -> None, no panic, comparison still succeeds
// ---------------------------------------------------------------------------

#[cfg(unix)]
#[test]
fn non_utf8_file_name_yields_none_display_name_without_panic() {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;

    let root = unique_root("utf8");
    std::fs::create_dir_all(&root).unwrap();

    // "bad-" + an invalid UTF-8 byte (0xFF is never valid in any UTF-8
    // sequence) + ".xlsx". Loudly fail construction rather than silently
    // falling back to a UTF-8-safe name, per the handoff's known risk.
    let mut raw = b"bad-".to_vec();
    raw.push(0xFF);
    raw.extend_from_slice(b".xlsx");
    let file_name = OsString::from_vec(raw);
    assert!(
        file_name.to_str().is_none(),
        "test construction bug: the chosen byte sequence is valid UTF-8 \
         after all, so this test would not exercise the non-UTF-8 path"
    );

    let old_path = root.join(&file_name);
    let new_path = root.join("new.xlsx");
    std::fs::write(&old_path, wb_empty())
        .unwrap_or_else(|e| panic!("filesystem rejected a non-UTF-8 file name ({e}) -- this test cannot run on this filesystem; do not silently skip, report it"));
    std::fs::write(&new_path, wb_empty()).unwrap();

    // No panic: the comparison itself is the assertion that nothing inside
    // `open_path` does `to_str().unwrap()` on the path.
    let diff = compare_paths(&old_path, &new_path).unwrap();

    assert_eq!(
        diff.old.source.display_name, None,
        "non-UTF-8 file name must yield None, not a lossy or truncated string"
    );
    assert_eq!(diff.new.source.display_name.as_deref(), Some("new.xlsx"));

    std::fs::remove_dir_all(&root).ok();
}

// ---------------------------------------------------------------------------
// 4: error paths do not leak the directory either
// ---------------------------------------------------------------------------

#[test]
fn open_error_on_a_nested_path_does_not_leak_the_directory() {
    let root = unique_root("err");
    let dir = root.join(SENSITIVE_DIR);
    std::fs::create_dir_all(&dir).unwrap();
    // Deliberately do not create this file -- NotFound is the simplest
    // reliable way to trigger OpenWorkbook without needing a corrupt fixture.
    let missing = dir.join("missing.xlsx");
    let new_path = dir.join("new.xlsx");
    std::fs::write(&new_path, wb_empty()).unwrap();

    let err = compare_paths(&missing, &new_path).unwrap_err();
    let rendered = err.to_string();

    assert!(
        !rendered.contains(SENSITIVE_DIR),
        "error message leaked the parent directory: {rendered}"
    );
    assert!(
        !rendered.contains(root.to_string_lossy().as_ref()),
        "error message leaked the full temp root: {rendered}"
    );
    // Positive shape: the file name (or the documented "<unknown>"
    // fallback) is what actually appears, not merely "the directory is
    // absent because the whole message is empty".
    assert!(
        rendered.contains("missing.xlsx") || rendered.contains("<unknown>"),
        "expected the file name or the '<unknown>' fallback, got: {rendered}"
    );

    std::fs::remove_dir_all(&root).ok();
}

// ---------------------------------------------------------------------------
// 5: byte and reader inputs carry no path at all
// ---------------------------------------------------------------------------

#[test]
fn byte_inputs_carry_no_display_name() {
    let old = wb_empty();
    let new = wb_empty();
    let diff = compare_bytes(&old, &new).unwrap();
    assert_eq!(diff.old.source.display_name, None);
    assert_eq!(diff.new.source.display_name, None);
}

#[test]
fn reader_inputs_carry_no_display_name() {
    let old = std::io::Cursor::new(wb_empty());
    let new = std::io::Cursor::new(wb_empty());
    let diff = compare_readers(old, new).unwrap();
    assert_eq!(diff.old.source.display_name, None);
    assert_eq!(diff.new.source.display_name, None);
}
