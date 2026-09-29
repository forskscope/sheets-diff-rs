# RFC-028 — Fuzzing and Hostile-Input Hardening

**Status.** Implemented (2.0.0–2.2.3) — verified 2026-08-16 against the implementation.
**Target:** v2.0+  
**Related:** RFC-004, RFC-005, RFC-016, RFC-026

**Corrected M9 unit 01 (unreleased, "annotate, do not downgrade" — the same ruling M10 unit 01 gave RFC-009).**
`fuzz_open_xlsx_bytes` split its one input at `data.len() / 2`, so a valid seed had to be two
workbooks concatenated **and of exactly equal length**; every seed in the corpus failed at
`not an xlsx file`, and **no code past the ZIP-header check had ever been fuzzed** — not the sheet
reader, the normaliser, the comparer, or alignment. f123, f130 and f131 all lived in exactly that
code. The framing is now length-prefixed (`fuzz/src/framing.rs`) and a second target,
`fuzz_self_comparison`, compares a workbook with itself — an oracle (zero cell diffs, in every
alignment mode) rather than "did not panic", and one with a demonstrated capacity to fail (see §10).
**Reaching the reader for the first time found three real defects, and a fourth was found while
fixing the third; three are fixed (f132, f133, f134) and one remains open — the `calamine` overflow,
upstream #694. See §7's annotation and `fuzz/corpus-quarantine/README.md`.** §7's contract is about this
crate's own public API (`compare_bytes` must not panic), not about which frame in the backtrace is
whose: naming a dependency's bug does not discharge a promise about the API a caller actually holds,
and the third defect found this way is not even a dependency's — it is `src/matcher.rs`'s own.

## 1. Summary

Define a fuzzing and hostile-input strategy for `.xlsx` input handling. A diff
library embedded in GUI applications must treat malformed files as ordinary
untrusted input, not as exceptional process-crashing events.

## 2. Motivation

`.xlsx` files are ZIP packages containing XML and related parts. They may be
corrupt, truncated, maliciously large, password-protected, or crafted to trigger
parser edge cases. Even if low-level parsing is delegated to `calamine`,
`sheets-diff` owns its error handling, resource limits, and no-panic contract.

## 3. Goals

- Verify malformed inputs return errors or diagnostics, not panics.
- Exercise path, bytes, and reader APIs.
- Test resource-limit behavior.
- Add fuzz targets for address conversion, range merging, typed value
  normalization, and workbook opening where feasible.

## 4. Non-goals

- Replacing calamine's parser.
- Guaranteeing safety against all decompression bombs in v2.0.
- Opening encrypted workbooks.

## 5. Fuzz targets

Suggested targets:

```text
fuzz_addr_roundtrip
fuzz_range_merge
fuzz_cell_value_normalization
fuzz_diff_options_builder
fuzz_open_xlsx_bytes
fuzz_sheet_matching_manifest
```

`fuzz_open_xlsx_bytes` should be behind an optional fuzzing setup because parser
fuzzing can be expensive.

**Annotated M9 unit 01 (unreleased).** Four of six exist (`fuzz_addr_roundtrip`, `fuzz_range_merge`,
`fuzz_diff_options_builder`, `fuzz_open_xlsx_bytes`); `fuzz_cell_value_normalization` and
`fuzz_sheet_matching_manifest` do not and are out of this unit's scope. A fifth,
`fuzz_self_comparison`, exists and was not suggested here — it reaches past the archive with an
oracle (§10). This is a sketch ("suggested"), not a contract; the contract is §10, and §10 is met.

## 6. Corpus seeds

Seed corpus should include:

- empty file;
- random bytes;
- valid ZIP but not XLSX;
- truncated XLSX;
- password-protected workbook if fixture licensing allows;
- workbook with empty sheets;
- workbook with wide columns;
- workbook with many sheets;
- workbook with formulas;
- workbook with unsupported objects.

**Annotated M9 unit 01 (unreleased).** All ten now exist, split across two corpora
(`fuzz/corpus/fuzz_open_xlsx_bytes/`, `fuzz/corpus/fuzz_self_comparison/`) — see the review request's
category table for which seed is which. Through this unit only the first two existed, and the
target's own framing (see the note at the top of this RFC) made writing the other eight
impractical: a seed had to be two workbooks of exactly equal length.

## 7. Panic policy

Public APIs must not panic on malformed input. Panics in internal debug asserts
are acceptable only when unreachable by public ordinary input and should not be
used for parser errors.

Use `Result` and diagnostics consistently.

**Annotated M9 unit 01, updated by f132 (both unreleased) — this policy was VIOLATED by two inputs
through `compare_bytes`; a third, distinct violation of a different policy (§*Panic policy* is silent
on wrong-but-non-panicking answers) was found by the same work.** All three were found the first time
a fuzz target reached past the ZIP header:

1. **Fixed (f132).** A 512-byte file caused a single 9,261,285,372-byte allocation and aborted the
   process. `Xlsx::new` called `check_for_password_protected` unconditionally, which parsed the input
   as a CFB container; a DIFAT-sector-count field in the header reached `Vec::with_capacity` with no
   check against the file's actual length (`calamine-0.36.1/src/xlsx/mod.rs:2939` → `cfb.rs:260`,
   field read at `cfb.rs:224`). **`max_input_bytes` could not catch it** — the size came from a header
   field, not the input's length — and **`Limits::hardened()` did not prevent it**, verified. Closed
   at `src/open.rs`: a `.xlsx` is a ZIP archive, and `open_workbook_from_cursor`'s call site was a
   single choke point, so anything not beginning with the ZIP magic is declined before `calamine`
   sees it — with a byte-scan carve-out, never a parse, for a real encrypted `.xlsx`, since our own
   encrypted-workbook fixture, the crash artifact and legacy `.xls` all begin the CFB magic
   (`d0cf11e0a1b11ae1`), so "reject CFB" and "report `EncryptedWorkbook`" could not be the same rule.
2. **Open.** A crafted worksheet panics in any build with debug assertions on — "attempt to multiply
   with overflow" in `get_row_and_optional_column` (`xlsx/mod.rs:2838`), parsing a base-26
   column-letter run with no length bound. Release builds wrap instead; on the artifact in hand the
   wrapped value then fails to parse and `compare_bytes` returns a clean `Err(sheet is malformed)`.
   **Debug builds are not an edge case** — every downstream `cargo test` is one. Not reachable by (1)'s
   pre-screen (it lives inside a valid archive); upstream-only unless that changes, and tracked as
   [calamine#694](https://github.com/tafia/calamine/issues/694) — filed by someone else in July, not
   by us. Defect 1 is [calamine#714](https://github.com/tafia/calamine/issues/714). **f132 measured
   it as reachable within CI's own fuzzing budget**, 3 of 20 `-runs=20000` sessions, once the
   encrypted-workbook seed is restored to active fuzzing — see `fuzz/corpus-quarantine/README.md`.
3. **Open, and ours.** `src/matcher.rs`'s exact-name matching pairs each old sheet with the *first*
   new sheet of the same name, without checking it was not already claimed by an earlier old sheet.
   Two old sheets sharing a (here, corrupted-to-empty) name make one new sheet double-matched and
   another silently reported `Added` instead of `Unchanged` — a workbook compared with itself reports
   changes that do not exist. **Not a panic — a silent wrong answer**, found by `fuzz_self_comparison`'s
   oracle rather than by a crash. Reproduces deterministically outside the fuzzer. Needs its own unit.

Until (2) and (3) are closed, the threat model's assurance row for "no panic on malformed input"
reads **Partially**, not "No" and not "Yes" — (1)'s fix moved it off "No"; (2) keeps it off "Yes". (3)
is a different property this section does not name; see the threat model's
*Normalisation/alignment/formula-attachment correctness* row.

## 8. Resource hardening

Fuzz and tests should cover:

- maximum sheet count;
- maximum cell count;
- maximum returned diff count;
- cancellation during long comparison;
- large shared string tables if the reader exposes them.

**Annotated M9 unit 01 (unreleased) — unmet, stated here because this is an imperative requirement,
not a sketch.** `fuzz_self_comparison` drives `AlignmentMode` and a `Limits::hardened()`-bounded
subset of `Limits` from its fuzz input (`fuzz/src/self_seed.rs`), covering the first two items:
maximum sheet count and maximum cell count are both `hardened()` fields, exercised whenever a
generated workbook's size approaches them. **The third and fourth are not covered, and the fourth
cannot be, by construction:**
- *Maximum returned diff count* (`max_diffs_returned`) is not driven by either target; adding it is
  in scope for a future unit and is a small, additive change to `self_seed.rs`.
- *Cancellation during a long comparison* needs a second thread to trip the token mid-comparison
  (`tests/alignment_cancellation.rs` does exactly this, outside fuzzing). **A libFuzzer target has no
  second thread of its own to drive one from** — `fuzz_target!` runs the harness function to
  completion on the calling thread, and spawning a thread inside the harness to race a cancellation
  against a comparison the fuzzer controls the size of would make the target's own behaviour
  non-deterministic input-to-input, which coverage-guided fuzzing depends on not being. The honest
  answer is that this item is not fuzzable as stated; it is tested by `tests/alignment_cancellation.rs`
  instead, which is where the requirement is actually met.
- *Large shared string tables*: not investigated this unit; out of scope, not named as covered.

## 9. CI integration

- Normal CI runs regression fixtures.
- Fuzz targets compile in CI.
- Nightly/manual job runs fuzzing for a time budget.
- Crashes create minimized corpus entries.

## 10. Acceptance criteria

- Malformed bytes through `try_from_bytes` never panic in regression tests.
- Address/range fuzz targets run without panics.
- At least one fuzz target is documented for maintainers.
- Security policy states that files are untrusted input and external links are
  never followed.

**Annotated M9 unit 01 (unreleased).** `try_from_bytes` names an API that no longer exists
(`grep -rn try_from_bytes src/` finds nothing) — pre-2.0 wording, not a regression, not rewritten
here. The equivalent malformed-bytes property (`compare_bytes` must not panic) is what
`fuzz_open_xlsx_bytes` fuzzes, and both fuzz targets are documented in `fuzz/README.md`.
`fuzz_self_comparison` adds a second, stronger criterion this RFC did not ask for: not merely "did
not panic" but "gave the only correct answer" — and, reaching the reader for the first time, found
two crashes in `calamine` neither fuzz target nor this RFC's acceptance criteria had ever exercised
(unfixed; see the review request). Nothing here claims those crashes as covered — they are the
newly-found gap, not the closed criterion.
