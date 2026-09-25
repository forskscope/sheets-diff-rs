# RFC-013: Output Formatters, CLI, and Exit Codes

**Status.** Implemented (2.0.0–2.4.x) — verified 2026-08-16; the deferral closed 2026-08-17 (M4 Handoff 03). Exit code 3 is emitted for invalid/corrupt input: `exit_code_for` in `src/main.rs` maps `OpenWorkbook{NotXlsx|Corrupt}`, `ReadSheet{SheetNotFound|MalformedSheet}`, `UnsupportedFormat` and `EncryptedWorkbook` to 3, narrowing 2 to environment, caller, limit and internal errors. Covered by six subprocess tests in `tests/cli.rs`.
**Corrected M8 unit 03 (2.6.0):** this Status was incomplete. §5 specifies
`sheets-diff --format json`, and it was never wired — the CLI offered `summary` and
`unified` only. The verification pass read this RFC and did not notice a specified
CLI format that did not exist. It now does (`OutputFormat::Json`, dispatching to
`to_json_pretty`), and the `cli` feature enables `serde` so that the binary's
advertised formats do not depend on how it was compiled. §5's names and signature and
§11's open question are corrected in place below, each annotated with what it said.
**Target:** v2.0.0  
**Created:** 2026-06-11  
**Category:** Output/CLI  

## 1. Summary

Rebuild text and CLI output as adapters over the public v2 library model.

## 2. Motivation

The CLI remains useful, but the library is the primary product. Text/unified output should be generated from `WorkbookDiff`, not from a separate privileged implementation path.

## 3. Goals

- Keep a simple CLI for users.
- Implement CLI through public library API only.
- Provide human summary output.
- Provide unified/text output where practical.
- Return stable exit codes.

## 4. Non-goals

- Do not make CLI output the canonical model.
- Do not require CLI features for library users.
- Do not implement interactive merge UI.

## 5. External design

Proposed output module:

```rust
pub mod output {
    pub fn render_summary(diff: &WorkbookDiff) -> String;
    pub fn render_unified(diff: &WorkbookDiff, options: UnifiedOutputOptions) -> String;
    #[cfg(feature = "serde")]
    pub fn to_json(diff: &WorkbookDiff) -> Result<String, String>;
    #[cfg(feature = "serde")]
    pub fn to_json_pretty(diff: &WorkbookDiff) -> Result<String, String>;
}
```

> **Corrected M8 unit 03.** This block specified `#[cfg(feature = "json")]` and a
> function `render_json` returning `Result<String, serde_json::Error>`. There is no
> `json` feature — it is `serde` — and the functions are `output::json::to_json` and
> `to_json_pretty`, returning `Result<String, String>`. The spec and the code had
> drifted in name and in signature. The RFC now matches the code, not the reverse; the
> error type is not changed here. (`render_unified` also differs from this sketch: it
> takes no `UnifiedOutputOptions`.)

CLI examples:

```text
sheets-diff old.xlsx new.xlsx
sheets-diff --format summary old.xlsx new.xlsx
sheets-diff --format unified old.xlsx new.xlsx
sheets-diff --format json old.xlsx new.xlsx
sheets-diff --no-formulas old.xlsx new.xlsx
sheets-diff --no-warnings --format json old.xlsx new.xlsx
```

Exit codes:

```text
0 = compared successfully and no differences
1 = compared successfully and differences found
2 = invalid command-line usage
3 = input/open/read error
4 = cancelled or limit exceeded
5 = internal error
```

## 6. Internal design

CLI implementation should live in `src/bin/sheets-diff.rs` or a small CLI module that imports the library crate as an external consumer would.

Text renderers should not mutate the model. They should be pure functions from `WorkbookDiff` to strings or writers.

## 7. Data lifecycle

1. CLI parses arguments into `DiffOptions`.
2. CLI calls public compare API.
3. CLI selects renderer.
4. CLI writes output.
5. CLI maps result/error to exit code.

## 8. Error, diagnostic, and edge-case behavior

Library renderer functions may return formatting errors only when writing to an external writer. String renderers should be infallible.

CLI may write to stdout/stderr; library core must not.

## 9. Testing and acceptance criteria

Acceptance criteria:

- CLI uses public APIs only.
- Exit code 1 is used for successful comparisons with differences.
- Invalid/corrupt inputs produce exit code 3.
- Existing unified-style output has a compatibility test where practical.
- JSON output is behind a feature if serde is optional. *(As built: `output::json` is
  behind `serde`; the CLI binary's `--format json` is always present because the `cli`
  feature enables `serde`.)*

## 10. Migration and compatibility

v1 CLI users should receive similar basic behavior, but output wording may change. Document exit code changes explicitly.

## 11. Open questions

- Should CLI be in the same crate or split into `sheets-diff-cli`?
- Should JSON output be stabilized in v2.0 or marked experimental?
  **Answered (M8 unit 03, deliberately — shipping `--format json` had answered it by
  default):** the JSON shape is **stable within 2.x**. The model types are public and
  `#[non_exhaustive]`, so a minor release may **add** fields or enum variants, and a
  consumer must ignore what it does not know; no existing field or variant name is
  renamed or removed within a major version. The *values* are the model's. For a
  **library** consumer, `CellDateTime.iso` is populated only in a build with the
  `chrono` feature and is `null` otherwise. For the **installed command-line tool** it
  is always populated: `cli` enables `chrono`.
  *(Corrected M8 unit 06, before `--format json` was published. Unit 03 first recorded
  that the CLI's `iso` depended on how the binary was compiled — `null` under
  `--features cli`, the documented install command's own build. That was the
  surface this RFC's §5 example promised and could not deliver; `cli` now enables
  `chrono`, and the exception is narrowed to the library, not deleted.)*
