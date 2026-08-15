# RFC-030 — Extended Fixture Generators and Corpus Management

**Status.** Implemented (2.0.0–2.2.3) — restored 2026-08-15; not individually re-verified against the implementation.
**Target:** v2.0+  
**Related:** RFC-015, RFC-027, RFC-028

## 1. Summary

Define a maintainable test corpus and programmatic fixture-generation strategy.
Spreadsheet diff correctness requires many edge cases that are difficult to
maintain as hand-authored binary files.

## 2. Motivation

The v1 test suite relies on a small golden fixture. v2 must cover wide columns,
renamed sheets, formulas, typed values, corrupt input, non-UTF-8 paths,
empty/one-sided ranges, duplicate formula/value changes, and optional alignment.
Programmatic fixtures make it easier to reproduce bugs and share cases with
downstream integrators.

## 3. Goals

- Generate deterministic `.xlsx` fixtures.
- Keep binary fixtures small and reviewable where possible.
- Store scenario metadata next to expected outputs.
- Allow downstream apps to contribute cases without exposing private data.

## 4. Non-goals

- Depending on Excel itself for fixture generation.
- Maintaining huge real-world workbook corpora in the repository.
- Testing every OpenXML feature.

## 5. Fixture layout

```text
tests/fixtures/
  generated/
    wide_columns/
      old.xlsx
      new.xlsx
      expected.json
      scenario.toml
    renamed_sheet/
      old.xlsx
      new.xlsx
      expected.json
      scenario.toml
  malformed/
    not_zip.bin
    truncated.xlsx
  corpus/
    README.md
```

## 6. Scenario metadata

```toml
name = "wide_columns_xfd"
kind = "regression"
old = "old.xlsx"
new = "new.xlsx"
options = { compare_formulas = true }
expected = "expected.json"
notes = "Covers A1 addressing through XFD."
```

## 7. Generator design

Create a small internal fixture generator crate or xtask:

```text
xtask fixtures generate
xtask fixtures verify
xtask fixtures bless
```

The generator should use a Rust `.xlsx` writer crate if practical. If writer
support is insufficient for some scenarios, minimal checked-in binary fixtures
are acceptable.

## 8. Golden output policy

Expected outputs should use the stable JSON schema when the `serde` feature is
enabled. Text output golden files are useful but secondary.

## 9. Privacy policy

Do not accept real customer workbooks into the public corpus unless they are
fully synthetic or explicitly sanitized. Prefer reduced fixtures generated from
bug reports.

## 10. Acceptance criteria

- Fixture generation is reproducible.
- Wide-column, corrupt-file, typed-value, formula, renamed-sheet, and empty-sheet
  scenarios exist before v2.0.
- Each regression bug adds a scenario.
- Fixture docs explain how downstream apps can contribute a minimal reproducer.
