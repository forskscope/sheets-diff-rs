//! CLI subprocess tests (RFC-013): the `sheets-diff` binary's exit-code
//! contract. Cover every code the CLI can produce, not only the new one --
//! this closes the gap where exit codes had never been verified by anything.
#![cfg(feature = "cli")]

mod support;
use support::wb_sheets;

use std::path::{Path, PathBuf};
use std::process::Command;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_sheets-diff"))
}

fn fixture(name: &str, file: &str) -> String {
    Path::new("tests/fixtures/generated")
        .join(name)
        .join(file)
        .to_string_lossy()
        .into_owned()
}

#[test]
fn exit_0_when_no_differences() {
    let old = fixture("empty_sheet", "old.xlsx");
    let new = fixture("empty_sheet", "new.xlsx");
    let status = bin().args([&old, &new]).status().unwrap();
    assert_eq!(status.code(), Some(0));
}

#[test]
fn exit_1_when_differences_found() {
    let old = fixture("sparse_range", "old.xlsx");
    let new = fixture("sparse_range", "new.xlsx");
    let status = bin().args([&old, &new]).status().unwrap();
    assert_eq!(status.code(), Some(1));
}

#[test]
fn exit_2_for_invalid_cli_options() {
    // clap rejects an unrecognised --format value before any comparison runs.
    let old = fixture("empty_sheet", "old.xlsx");
    let new = fixture("empty_sheet", "new.xlsx");
    let status = bin()
        .args([old.as_str(), new.as_str(), "--format", "not-a-real-format"])
        .status()
        .unwrap();
    assert_eq!(status.code(), Some(2));
}

#[test]
fn exit_2_for_missing_file() {
    // NotFound is an environment condition, not "corrupt input" -- stays 2.
    let new = fixture("empty_sheet", "new.xlsx");
    let status = bin()
        .args(["tests/fixtures/does-not-exist.xlsx", new.as_str()])
        .status()
        .unwrap();
    assert_eq!(status.code(), Some(2));
}

#[test]
fn exit_3_for_corrupt_input() {
    let new = fixture("empty_sheet", "new.xlsx");
    let status = bin()
        .args(["tests/fixtures/corrupt/not_a_zip.xlsx", new.as_str()])
        .status()
        .unwrap();
    assert_eq!(status.code(), Some(3));
}

#[test]
fn exit_3_for_encrypted_workbook() {
    // M4 unit 03 mapped EncryptedWorkbook => 3 deliberately (RFC-032: the
    // tool cannot proceed with the file as given, for reasons intrinsic to
    // it, same bucket as corrupt/wrong-format input). Unprotected until now.
    let new = fixture("empty_sheet", "new.xlsx");
    let status = bin()
        .args(["tests/fixtures/corrupt/encrypted.xlsx", new.as_str()])
        .status()
        .unwrap();
    assert_eq!(status.code(), Some(3));
}

// --- M8 unit 01: structural-only differences ------------------------------------
//
// A reordered or renamed sheet is a difference the engine records and the CLI used
// to hide (reorder: exit 0) or half-hide (rename: exit 1, empty unified output).
// The pairs are built here from `support::wb_sheets`, with identical cells on both
// sides, so nothing but the structure differs.

const CELL_A: &[(u32, u16, &str)] = &[(0, 0, "a")];
const CELL_B: &[(u32, u16, &str)] = &[(0, 0, "b")];

/// Writes `old.xlsx` and `new.xlsx` into a fresh per-test directory.
fn write_pair(tag: &str, old: &[u8], new: &[u8]) -> (PathBuf, PathBuf, PathBuf) {
    let dir = std::env::temp_dir().join(format!("sheets-diff-cli-{tag}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let (o, n) = (dir.join("old.xlsx"), dir.join("new.xlsx"));
    std::fs::write(&o, old).unwrap();
    std::fs::write(&n, new).unwrap();
    (dir, o, n)
}

/// Runs the CLI; returns `(exit code, stdout)`.
fn run(old: &Path, new: &Path, extra: &[&str]) -> (Option<i32>, String) {
    let out = bin().arg(old).arg(new).args(extra).output().unwrap();
    (out.status.code(), String::from_utf8(out.stdout).unwrap())
}

fn reordered() -> (Vec<u8>, Vec<u8>) {
    (
        wb_sheets(&[("Alpha", CELL_A), ("Beta", CELL_B)]),
        wb_sheets(&[("Beta", CELL_B), ("Alpha", CELL_A)]),
    )
}

fn renamed() -> (Vec<u8>, Vec<u8>) {
    (
        wb_sheets(&[("Before", CELL_A)]),
        wb_sheets(&[("After", CELL_A)]),
    )
}

#[test]
fn exit_1_for_a_pure_reorder() {
    let (old, new) = reordered();
    let (dir, o, n) = write_pair("reorder-exit", &old, &new);
    let (code, stdout) = run(&o, &n, &[]);
    std::fs::remove_dir_all(&dir).ok();
    assert_eq!(
        code,
        Some(1),
        "a reordered workbook is a difference; stdout: {stdout}"
    );
}

#[test]
fn exit_1_for_a_pure_rename() {
    let (old, new) = renamed();
    let (dir, o, n) = write_pair("rename-exit", &old, &new);
    let (code, _) = run(&o, &n, &[]);
    std::fs::remove_dir_all(&dir).ok();
    assert_eq!(code, Some(1));
}

#[test]
fn exit_0_for_an_identical_pair_with_several_sheets() {
    // The guard against fixing the reorder by making everything exit 1.
    let bytes = wb_sheets(&[("Alpha", CELL_A), ("Beta", CELL_B)]);
    let (dir, o, n) = write_pair("identical-exit", &bytes, &bytes);
    let (code, _) = run(&o, &n, &[]);
    let (code_unified, unified) = run(&o, &n, &["--format", "unified"]);
    std::fs::remove_dir_all(&dir).ok();
    assert_eq!(code, Some(0));
    assert_eq!(code_unified, Some(0));
    assert_eq!(unified, "--- old.xlsx\n+++ new.xlsx\n", "nothing to render");
}

#[test]
fn unified_output_renders_a_pure_reorder() {
    let (old, new) = reordered();
    let (dir, o, n) = write_pair("reorder-unified", &old, &new);
    let (code, stdout) = run(&o, &n, &["--format", "unified"]);
    std::fs::remove_dir_all(&dir).ok();
    assert_eq!(code, Some(1));
    assert!(stdout.contains("[moved: position 1 → 2]"), "got: {stdout}");
    assert!(stdout.contains("[moved: position 2 → 1]"), "got: {stdout}");
}

#[test]
fn unified_output_renders_a_pure_rename() {
    // Before: exit 1 and the entire output was the two header lines.
    let (old, new) = renamed();
    let (dir, o, n) = write_pair("rename-unified", &old, &new);
    let (code, stdout) = run(&o, &n, &["--format", "unified"]);
    std::fs::remove_dir_all(&dir).ok();
    assert_eq!(code, Some(1));
    assert!(
        stdout.contains("[renamed: 'Before' → 'After']"),
        "got: {stdout}"
    );
}
