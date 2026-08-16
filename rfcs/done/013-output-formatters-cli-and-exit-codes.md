# RFC-013: Output Formatters, CLI, and Exit Codes

**Status.** Partially implemented (2.0.0–2.2.3) — verified 2026-08-16. Deferred: exit code 3 (invalid/corrupt input) is never emitted — `src/main.rs` uses exit code 2 for both invalid CLI options and comparison errors; no exit code 3 exists anywhere in the CLI.
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
    #[cfg(feature = "json")]
    pub fn render_json(diff: &WorkbookDiff) -> Result<String, serde_json::Error>;
}
```

CLI examples:

```text
sheets-diff old.xlsx new.xlsx
sheets-diff --format summary old.xlsx new.xlsx
sheets-diff --format unified old.xlsx new.xlsx
sheets-diff --format json old.xlsx new.xlsx
sheets-diff --no-formulas old.xlsx new.xlsx
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
- JSON output is behind a feature if serde is optional.

## 10. Migration and compatibility

v1 CLI users should receive similar basic behavior, but output wording may change. Document exit code changes explicitly.

## 11. Open questions

- Should CLI be in the same crate or split into `sheets-diff-cli`?
- Should JSON output be stabilized in v2.0 or marked experimental?
