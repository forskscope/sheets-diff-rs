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

// --- M8 unit 02: `--no-warnings` ---------------------------------------------------
//
// The flag is a *display* control: it hides the diagnostics section of the output
// and changes nothing else. It is not `min_severity`, which is a *collection*
// filter that would change the counts (and so would make `--no-warnings` report
// zero warnings for a workbook that has them). `chart_sheet` is the corpus
// scenario with real warnings: a chart sheet on each side is not compared.

fn chart_sheet() -> (String, String) {
    (
        fixture("chart_sheet", "old.xlsx"),
        fixture("chart_sheet", "new.xlsx"),
    )
}

fn run_paths(old: &str, new: &str, extra: &[&str]) -> (Option<i32>, String, String) {
    let out = bin().args([old, new]).args(extra).output().unwrap();
    (
        out.status.code(),
        String::from_utf8(out.stdout).unwrap(),
        String::from_utf8(out.stderr).unwrap(),
    )
}

#[test]
fn unified_output_shows_the_diagnostics_section_and_no_warnings_hides_it() {
    let (old, new) = chart_sheet();
    let (code, with, _) = run_paths(&old, &new, &["--format", "unified"]);
    let (code_hidden, without, _) =
        run_paths(&old, &new, &["--format", "unified", "--no-warnings"]);

    assert!(
        with.contains("# Diagnostics"),
        "precondition — no section: {with}"
    );
    assert!(with.contains("unsupported_workbook_feature"), "got: {with}");
    assert!(
        !without.contains("# Diagnostics"),
        "the section survived --no-warnings: {without}"
    );
    assert!(
        !without.contains("unsupported_workbook_feature"),
        "got: {without}"
    );
    // The flag hides a section; it changes neither the exit code nor the diff itself.
    assert_eq!(code, code_hidden);
    assert_eq!(
        with.split("# Diagnostics").next().unwrap().trim_end(),
        without.trim_end(),
        "everything above the diagnostics section must be identical"
    );
}

#[test]
fn no_warnings_leaves_every_summary_count_unchanged() {
    // The test that distinguishes this implementation from "--no-warnings is
    // min_severity = Error", which would print `0 warning(s)` here.
    let (old, new) = chart_sheet();
    let (_, with, _) = run_paths(&old, &new, &[]);
    let (_, without, _) = run_paths(&old, &new, &["--no-warnings"]);

    assert!(
        with.contains("warning(s)") && !with.contains("0 error(s), 0 warning(s)"),
        "precondition — the workbook must actually have warnings: {with}"
    );
    assert_eq!(with, without, "--no-warnings changed the summary output");
}

#[test]
fn no_warnings_does_not_silence_stderr() {
    // Diagnostics are not stderr, and an error is not a warning.
    let new = fixture("empty_sheet", "new.xlsx");
    let (code, stdout, stderr) = run_paths(
        "tests/fixtures/does-not-exist.xlsx",
        &new,
        &["--no-warnings"],
    );
    assert_eq!(code, Some(2));
    assert!(stdout.is_empty(), "got: {stdout}");
    assert!(
        stderr.contains("cannot open"),
        "the error must still be reported, got: {stderr}"
    );
}

// --- M8 unit 03: `--format json` ---------------------------------------------------
//
// RFC-013 specified it; the library half (`to_json_pretty`) existed; the CLI had no
// such format. The output is parsed as JSON in every test, never string-matched: a
// test that greps the text would pass on output no consumer could parse.

use serde_json::Value;

fn json_of(stdout: &str) -> Value {
    serde_json::from_str(stdout)
        .unwrap_or_else(|e| panic!("stdout is not JSON ({e}); it was:\n{stdout}"))
}

fn json_run(old: &str, new: &str, extra: &[&str]) -> (Option<i32>, String, String) {
    let mut args = vec!["--format", "json"];
    args.extend_from_slice(extra);
    run_paths(old, new, &args)
}

#[test]
fn json_on_a_differing_pair_parses_and_exits_1() {
    let (old, new) = (
        fixture("sparse_range", "old.xlsx"),
        fixture("sparse_range", "new.xlsx"),
    );
    let (code, stdout, _) = json_run(&old, &new, &[]);
    assert_eq!(code, Some(1));
    let v = json_of(&stdout);
    assert_eq!(v["summary"]["cells_changed"], 1);
}

#[test]
fn json_on_an_identical_pair_parses_and_exits_0() {
    let (old, new) = (
        fixture("empty_sheet", "old.xlsx"),
        fixture("empty_sheet", "new.xlsx"),
    );
    let (code, stdout, _) = json_run(&old, &new, &[]);
    assert_eq!(code, Some(0));
    assert_eq!(json_of(&stdout)["summary"]["cells_changed"], 0);
}

#[test]
fn json_on_a_missing_file_exits_2_with_empty_stdout_and_the_message_on_stderr() {
    let new = fixture("empty_sheet", "new.xlsx");
    let (code, stdout, stderr) = json_run("tests/fixtures/does-not-exist.xlsx", &new, &[]);
    assert_eq!(code, Some(2));
    assert!(
        stdout.is_empty(),
        "stdout must be empty on an error, got: {stdout}"
    );
    assert!(stderr.contains("cannot open"), "got: {stderr}");
}

#[test]
fn json_on_a_corrupt_or_encrypted_workbook_exits_3_with_empty_stdout() {
    let new = fixture("empty_sheet", "new.xlsx");
    for bad in ["not_a_zip.xlsx", "encrypted.xlsx"] {
        let (code, stdout, stderr) = json_run(&format!("tests/fixtures/corrupt/{bad}"), &new, &[]);
        assert_eq!(code, Some(3), "{bad}");
        assert!(
            stdout.is_empty(),
            "{bad}: stdout must be empty, got: {stdout}"
        );
        assert!(!stderr.is_empty(), "{bad}: the message belongs on stderr");
    }
}

#[test]
fn json_output_is_pretty_printed_and_is_only_json() {
    let (old, new) = (
        fixture("sparse_range", "old.xlsx"),
        fixture("sparse_range", "new.xlsx"),
    );
    let (_, stdout, _) = json_run(&old, &new, &[]);
    assert!(
        stdout.lines().count() > 20,
        "pretty output is line-oriented, got: {stdout}"
    );
    assert!(
        stdout.contains("\n  \""),
        "expected two-space indentation, got: {stdout}"
    );
    assert!(
        stdout.starts_with('{'),
        "nothing may precede the JSON: {stdout}"
    );
    assert!(
        stdout.ends_with("}\n"),
        "nothing may follow the JSON: {stdout:?}"
    );
    json_of(&stdout); // and the whole of stdout parses
}

/// `(cells_changed, sheets_moved)` and the sheet names, read from `--format summary`.
fn summary_facts(text: &str) -> (u64, u64, Vec<String>) {
    let num_before = |line: &str, word: &str| -> u64 {
        let head = line.split(word).next().unwrap();
        head.trim_end_matches([' ', ','])
            .rsplit([' ', ',', ':'])
            .next()
            .unwrap()
            .parse()
            .unwrap()
    };
    let cells = text
        .lines()
        .find(|l| l.trim_start().starts_with("cells"))
        .unwrap();
    let sheets = text
        .lines()
        // "sheets :" with the space: the title line `sheets-diff: a → b` also starts with "sheets".
        .find(|l| l.trim_start().starts_with("sheets :"))
        .unwrap();
    let names = text
        .lines()
        .filter_map(|l| l.trim_start().strip_prefix("sheet '"))
        .map(|rest| rest.split('\'').next().unwrap().to_string())
        .collect();
    (
        num_before(cells, " changed"),
        num_before(sheets, " moved"),
        names,
    )
}

#[test]
fn json_agrees_with_the_summary_of_the_same_comparison() {
    // `sheet_reordered`: Alpha and Beta swap tabs, Gamma has one changed cell.
    let (old, new) = (
        fixture("sheet_reordered", "old.xlsx"),
        fixture("sheet_reordered", "new.xlsx"),
    );
    let (_, summary, _) = run_paths(&old, &new, &["--format", "summary"]);
    let (cells, moved, names) = summary_facts(&summary);
    let (_, stdout, _) = json_run(&old, &new, &[]);
    let v = json_of(&stdout);

    assert_eq!(v["summary"]["cells_changed"], cells);
    assert_eq!(v["summary"]["sheets_moved"], moved);
    let mut json_names: Vec<String> = v["sheets"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|s| s["change"] != "Unchanged")
        .map(|s| s["new_sheet"]["name"].as_str().unwrap().to_string())
        .collect();
    let mut summary_names = names;
    json_names.sort();
    summary_names.sort();
    assert_eq!(json_names, summary_names, "the same sheets, by name");
    assert!(
        moved >= 2 && cells >= 1,
        "precondition — the fixture must exercise both: {summary}"
    );
}

#[test]
fn json_reports_a_pure_reorders_moved_sheets() {
    // Unit 01's case: nothing but the tab order differs.
    let old = wb_sheets(&[("Alpha", CELL_A), ("Beta", CELL_B)]);
    let new = wb_sheets(&[("Beta", CELL_B), ("Alpha", CELL_A)]);
    let (dir, o, n) = write_pair("json-reorder", &old, &new);
    let (code, stdout, _) = json_run(o.to_str().unwrap(), n.to_str().unwrap(), &[]);
    std::fs::remove_dir_all(&dir).ok();

    assert_eq!(code, Some(1));
    let v = json_of(&stdout);
    assert_eq!(v["summary"]["cells_changed"], 0);
    assert_eq!(v["summary"]["sheets_moved"], 2);
    let changes: Vec<&Value> = v["sheets"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| &s["change"])
        .collect();
    assert!(changes.iter().all(|c| *c == "Moved"), "got: {changes:?}");
}

#[test]
fn no_warnings_empties_the_json_diagnostics_arrays_and_keeps_the_counts() {
    // The interaction with unit 02. If JSON ignored the flag, `--no-warnings
    // --format json` would be inert — the defect this milestone exists to remove.
    let (old, new) = (
        fixture("chart_sheet", "old.xlsx"),
        fixture("chart_sheet", "new.xlsx"),
    );
    let (code, with_stdout, _) = json_run(&old, &new, &[]);
    let (code_hidden, without_stdout, _) = json_run(&old, &new, &["--no-warnings"]);
    let (with, without) = (json_of(&with_stdout), json_of(&without_stdout));

    assert!(
        !with["diagnostics"].as_array().unwrap().is_empty(),
        "precondition"
    );
    assert_eq!(without["diagnostics"], serde_json::json!([]));
    for sheet in without["sheets"].as_array().unwrap() {
        assert_eq!(
            sheet["diagnostics"],
            serde_json::json!([]),
            "a sheet kept its diagnostics"
        );
    }
    // The counters stay truthful, and so does everything else.
    assert_eq!(with["summary"], without["summary"]);
    assert_eq!(with["metrics"], without["metrics"]);
    assert_eq!(code, code_hidden);
}

#[test]
fn every_format_exits_the_same_way_for_the_same_inputs() {
    // The exit code is a property of the comparison, not of how it is displayed:
    // all four codes, three formats.
    let cases: [(&str, String, String, i32); 5] = [
        (
            "identical",
            fixture("empty_sheet", "old.xlsx"),
            fixture("empty_sheet", "new.xlsx"),
            0,
        ),
        (
            "differing",
            fixture("sparse_range", "old.xlsx"),
            fixture("sparse_range", "new.xlsx"),
            1,
        ),
        (
            "missing",
            "tests/fixtures/does-not-exist.xlsx".into(),
            fixture("empty_sheet", "new.xlsx"),
            2,
        ),
        (
            "corrupt",
            "tests/fixtures/corrupt/not_a_zip.xlsx".into(),
            fixture("empty_sheet", "new.xlsx"),
            3,
        ),
        (
            "encrypted",
            "tests/fixtures/corrupt/encrypted.xlsx".into(),
            fixture("empty_sheet", "new.xlsx"),
            3,
        ),
    ];
    for (name, old, new, expected) in cases {
        for fmt in ["summary", "unified", "json"] {
            let (code, _, _) = run_paths(&old, &new, &["--format", fmt]);
            assert_eq!(code, Some(expected), "{name} / --format {fmt}");
        }
    }
}

#[test]
fn help_lists_json() {
    let out = bin().arg("--help").output().unwrap();
    let help = String::from_utf8(out.stdout).unwrap();
    assert!(
        help.contains("json"),
        "--help must list the json format:\n{help}"
    );
}
