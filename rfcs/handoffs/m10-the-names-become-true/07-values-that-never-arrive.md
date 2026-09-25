# Handoff 07 — Values that never arrive, and a count that is always zero

**Governing.** [RFC-037 §3.8](../../accepted/037-v3-scope.md); RFC-005 (error
and diagnostic model); RFC-013 (CLI and exit codes)
**Roadmap.** M10 — unit 07
**Sequence.** Parallel with 06. Before 08.

## Purpose

Four public values nothing constructs. One of them makes the CLI print a number
that can only ever be zero, on every run.

**You found all four**, working unit 02, outside its scope, and reported rather
than absorbed them. This is them coming back as a unit.

## Background

Each appears only in its declaration, a `Display` arm, and an exit-code or
renderer arm.

### `Severity::Error` — structurally impossible, not merely unused

RFC-005's model is two-tier: **fatal** conditions are `SheetsDiffError`,
**recoverable** ones are `Diagnostic`. A diagnostic that is an "error" has no
room in that design. The closest real case — `DuplicateAlignmentKey`, "your rows
may have been paired wrongly" — is correctly a `Warning`. All eleven diagnostic
push sites emit `Info` or `Warning`.

**`render_summary` prints `diagnostics: N error(s), M warning(s)` and `N` can
only ever be 0.** A reader learns nothing from it and may reasonably infer that
errors are possible and they got lucky.

### The three error values

| Value | Why it goes |
|---|---|
| `SheetsDiffError::UnsupportedFormat` | duplicates `OpenWorkbook { kind: NotXlsx }`, which is what a non-`.xlsx` file actually produces. Two names for one condition. |
| `SheetsDiffError::Internal` | the escape hatch for our own bugs, never needed. `SheetsDiffError` is `#[non_exhaustive]`, so re-adding is a minor. |
| `OpenErrorKind::Locked` | see below — the only one that is not simply dead |

**`Locked` is a real condition we misclassify.** A file held open by Excel does
occur; `src/error.rs:259` maps io errors to `NotFound`, `PermissionDenied` or
`Other`, and nothing produces `Locked`, so a locked file is reported as
"permission denied" — nearly true. Detecting it properly needs raw per-platform
OS error codes, which is a **feature** and §4 forbids it here.

## Change scope

- `src/model.rs` — `Severity`, `DiagnosticSummary`
- `src/error.rs` — the three variants and their `Display` arms
- `src/main.rs` — the exit-code arms that become unreachable
- `src/output/text.rs` — the summary line and the `"ERROR"` prefix
- `tests/`
- `rfcs/done/005-*.md`, `rfcs/done/013-*.md`, `rfcs/done/033-*.md`
- `docs/` — anywhere the summary line's shape is shown
- `CHANGELOG.md`

## Non-change scope

- **Do not change which diagnostics are produced, or any severity assignment.**
  Nothing becomes a `Warning` that was an `Info`.
- **Do not add lock detection.** It is a feature.
- Do not change the exit-code *behaviour*: a non-`.xlsx` file still exits 3 via
  `NotXlsx`. Only the unreachable arm goes.
- Do not touch `Severity`'s `PartialOrd`/`Ord` derivation — `min_severity`'s
  `>=` depends on it and `Info < Warning` must still hold.

## Required implementation

1. **Remove `Severity::Error`**, `DiagnosticSummary::errors`, the `"ERROR"`
   prefix arm in `render_unified`, and the `N error(s)` half of
   `render_summary`'s line. The line becomes `diagnostics: M warning(s)`.
2. **Remove the three error values** and their `Display` arms and exit-code
   arms. **Check each exit-code arm before deleting it** — `main.rs`'s `_ => 2`
   catch-alls exist because the enums are `#[non_exhaustive]`, so removing a
   named arm must not silently reclassify anything that still occurs.
3. **`DiagnosticSummary`'s doc** says what it counts now, and that a diagnostic
   is never fatal — that is the design, and it is why there is no error count.
4. **RFC-005 records the two-tier reasoning explicitly**: fatal is
   `SheetsDiffError`, recoverable is `Diagnostic`, and therefore a diagnostic
   severity of "error" cannot exist. That sentence is why this is a removal
   rather than a gap to fill later.
5. **RFC-013's exit-code table** is checked against what can now be produced.
   Unit 03 of M8 reported (F-2) that this table already lists codes `4` and `5`
   that are never emitted — **that is not yours to fix here**, but do not make
   it worse, and say whether removing `UnsupportedFormat` touches it.

## Required tests

- **The summary line has no error count** — asserted on real output, so the
  format change is pinned.
- A comparison producing warnings reports them, and the count is the warnings
  only.
- **Exit codes unchanged**: a non-`.xlsx` file still exits 3, a missing file 2,
  a corrupt workbook 3. This is the guard on Non-change scope and it is the
  important one — the removals touch `exit_code_for`.
- `min_severity` still filters correctly with two severities, `Info < Warning`.

Removals demonstrate by compile failure; the output change does not. Show the
summary-line test failing first by a targeted change.

## Acceptance criteria

1. `Severity` has two variants; `DiagnosticSummary` has no `errors`.
2. `render_summary`'s line carries warnings only; `render_unified` has no
   unreachable prefix arm.
3. The three error values and their arms are gone.
4. **Exit codes for every input class are unchanged**, shown as a table against
   2.6.0.
5. RFC-005 carries the two-tier reasoning; RFC-033's lexicon agrees.
6. `min_severity` verified with the reduced enum.
7. Corpus byte-identical. **Goldens will move** — `summary.diagnostics` loses a
   field, so every golden carrying it changes. Show each differs only in that.
8. CHANGELOG `### Removed`, naming the serialised change: `summary.diagnostics`
   loses `errors`, and the CLI's summary line changes shape.
9. Gates green.

## Prohibited shortcuts

- **Do not keep `errors` as a field that is always 0.** That is the defect.
- Do not print `0 error(s)` conditionally. The count goes.
- Do not replace `Severity::Error` with a `Critical` or a `Fatal`. If a
  recoverable-but-severe condition is ever wanted, that is a design question and
  `Severity` is `#[non_exhaustive]`, so adding one later is a minor.
- Do not delete an exit-code arm without checking what now reaches the
  catch-all.

## Compatibility constraints

**Breaking, and the serialised surface moves.** `summary.diagnostics` loses
`errors` — `--format json` shipped in 2.6.0, so this is published,
machine-readable output. A consumer reading `errors` was reading a constant.

The CLI's summary line changes shape. Name it: someone parsing that line exists,
and they are the reason the number was worth removing rather than leaving.

## Known risks

- **`DiagnosticSummary` is `#[non_exhaustive]`**, so removing a field is a break
  but re-adding is additive. Say so — it is what makes this safe to decide now.
- **Goldens move.** Unlike unit 05's three, this touches every golden carrying
  `summary.diagnostics`. Count them and show each differs only in that field;
  this is the easiest place in the milestone to hide a second change.
- `docs/src/` executes and `semantics.md` asserts on diagnostic counts.

## Required evidence

- Compile-failure transcripts for the four removals
- The exit-code table, 2.6.0 vs now, every input class
- The summary-line before/after on real output
- Which goldens moved and proof of what changed in each
- Gates, each with exit status
- CI run link

## Review request format

Per development policy §9.2. Additionally: report the exit-code table yourself
rather than asserting no change, and say what now reaches each `_ =>` arm in
`exit_code_for`.
