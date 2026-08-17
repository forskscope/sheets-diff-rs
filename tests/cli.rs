//! CLI subprocess tests (RFC-013): the `sheets-diff` binary's exit-code
//! contract. Cover every code the CLI can produce, not only the new one --
//! this closes the gap where exit codes had never been verified by anything.
#![cfg(feature = "cli")]

use std::path::Path;
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
