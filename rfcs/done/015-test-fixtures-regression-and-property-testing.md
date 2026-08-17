# RFC-015: Test Fixtures, Regression, and Property Testing

**Status.** Implemented (2.0.0–2.4.x) — verified 2026-08-16; the deferral closed 2026-08-17 (M4 Handoff 03, extended by M5 Handoff 03). `tests/cli.rs` invokes the built binary via `Command::new(env!("CARGO_BIN_EXE_sheets-diff"))` and pins every exit code the CLI can produce — 0, 1, 2 (from two distinct sources) and 3 (corrupt input, and encrypted workbooks).
**Target:** v2.0.0 release gate  
**Created:** 2026-06-11  
**Category:** Testing  

## 1. Summary

Define the required fixture corpus, regression tests, property tests, and CI gates for v2.

## 2. Motivation

The known bugs and requested features are easy to regress without targeted fixtures. v2 must be built around generated and committed workbook fixtures that exercise wide columns, typed values, sheet renames, corrupt inputs, and large-workbook behavior.

## 3. Goals

- Create a broad `.xlsx` fixture corpus.
- Programmatically generate fixtures where practical.
- Add property tests for address conversion and ordering.
- Test public APIs, not only internals.
- Make v2 release impossible without regression coverage for known field issues.

## 4. Non-goals

- Do not rely only on one golden text fixture.
- Do not require proprietary Excel to generate tests.
- Do not check in huge fixtures unless needed.

## 5. External design

Fixture groups:

```text
fixtures/
  basic/
  corrupt/
  wide-columns/
  typed-values/
  formulas/
  sheet-renames/
  empty-sheets/
  sparse-ranges/
  duplicate-value-formula-changes/
  non-utf8-paths/        platform-specific tests
  large-generated/       ignored by default or generated on demand
```

Test categories:

- unit tests for pure utilities;
- integration tests through public compare APIs;
- property tests for coordinate/address conversion;
- golden JSON/text report tests;
- CLI tests for exit codes.

## 6. Internal design

Fixture generation should be in `tests/support` or `xtask`:

```text
cargo xtask generate-fixtures
cargo test
cargo test --features serde,json
cargo test -- --ignored large
```

If Rust `.xlsx` writing support is not reliable enough, keep minimal binary fixtures with clear provenance and add metadata explaining how they were made.

## 7. Data lifecycle

1. Fixture is generated or loaded.
2. Public API compares fixture pair.
3. Test asserts structured model fields.
4. Optional renderer test compares summary/unified/JSON output.
5. Regression test name references the bug class.

## 8. Error, diagnostic, and edge-case behavior

Corrupt input tests must assert errors, not panics. Large tests should be bounded and may run as ignored tests in normal CI.

Platform-specific non-UTF-8 path tests should be guarded appropriately.

## 9. Testing and acceptance criteria

Acceptance criteria:

- Known v1 address bug has regression tests.
- `try`/fallible bad-input behavior is tested.
- Typed value distinctions are tested.
- Rename detection is tested.
- Empty and one-sided ranges are tested.
- CLI exit codes are tested.
- CI runs default and feature combinations.

## 10. Migration and compatibility

No direct user migration, but the migration guide should mention that v2 behavior is fixture-backed and deterministic.

## 11. Open questions

- Which fixture generator crate should be used?
- Should large workbook tests run nightly instead of on every PR?
