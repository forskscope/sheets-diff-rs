# Handoff 01 — calamine 0.36 compatibility spike

**Governing RFC.** RFC-026 (dependency governance). Investigation only.
**Roadmap.** M2, decision D0
**Sequence.** First. Everything in M2 depends on what this finds.

## Purpose

Establish, with a build rather than an argument, whether `calamine` 0.36 is a
viable migration and what the true effective MSRV floor is.

**This unit lands no production code.** Its deliverable is a written report.

## Background

ForskScope has disabled `.xlsx` comparison over `RUSTSEC-2026-0194` /
`RUSTSEC-2026-0195` in `quick-xml` 0.39.4, reachable through `calamine` 0.35.
`calamine` 0.36.1 depends on `quick-xml` 0.41, the version that fixes both.

Roadmap decision D0 approved raising the MSRV from 1.85.0 to **1.88**, but
explicitly **contingent on a build proving 1.88 is actually sufficient**. That
contingency has never been discharged: an earlier attempt to run this spike was
blocked, so the number rests on `calamine` 0.36.1's *declared* `rust-version`
alone, not on the effective floor across its whole tree.

The architect has separately established, by reading the vendored source:
`calamine` 0.36.1 declares `rust-version = "1.88"`, depends on `quick-xml`
"0.41" and `zip` "8.6", keeps `mod formats` **private**, and exposes
`Xlsx::has_1904_epoch()`, `Hyperlink`, `expand_shared_formula`, merged regions,
tables and pivot tables. Treat all of that as a starting hypothesis to confirm
or refute, not as established fact.

## Change scope

Work in a **scratch clone or a throwaway branch**. Nothing from this unit is
merged.

## Non-change scope

Do **not** open a pull request, do **not** push to `main` or
`planning/*`, and do **not** commit a `Cargo.toml` change anywhere that CI will
build. If the spike needs to be preserved, push it to a clearly-named throwaway
branch and say so in the report.

## Questions to answer

Answer each with evidence — a command and its output — not an opinion.

### Q1 — Does it build at all?

Bump `calamine` to `0.36`, raise `rust-version` to `1.88`, and build. Report the
full error set if it does not, categorised into: our API usage changed, a
transitive dependency conflict, or an MSRV problem.

Remember the ordering trap: raise `rust-version` **before** bumping calamine, or
the MSRV-aware resolver may quietly hold you on 0.35 and produce a green build
with the advisory chain intact. Verify with `cargo tree` which version actually
resolved, and include that output.

### Q2 — What is the *effective* MSRV floor?

`calamine` declares 1.88, but the floor is the maximum across the whole tree,
including `zip` 8.6 and `quick-xml` 0.41. Determine it empirically: find the
lowest toolchain on which `cargo check --all-features` succeeds. Toolchains
1.85, 1.87, 1.88, 1.91 and 1.92 are installed locally.

**If the effective floor is above 1.88, stop and report.** D0 approved a
specific number; a higher one is an owner decision, not an implementation
detail.

### Q3 — What is the API delta against our usage?

Enumerate every calamine API this crate touches — `Xlsx::new`,
`worksheet_range`, `worksheet_formula`, `sheets_metadata`, `defined_names`,
`Data`, `CellErrorType`, `ExcelDateTime`, `XlsxError`, `SheetType`,
`SheetVisible` — and report for each whether it is unchanged, changed, or gone.

### Q4 — Does the advisory chain actually clear?

Confirm from `cargo tree` that `quick-xml` resolves to ≥ 0.41 and that 0.39.x is
absent from the tree. This is the whole point of the migration; verify it
directly rather than inferring it from the calamine version.

### Q5 — Does `has_1904_epoch()` let us fix `is_1904`?

`normalize.rs` currently hardcodes `is_1904: false`, which makes
`DateComparePolicy::NormalizeEquivalentDateTimes` dead code. Report whether
`Xlsx::has_1904_epoch()` is reachable at the point where we normalise cells —
we normalise per-cell inside `read_sheet_cells` while holding `&mut
OpenedWorkbook`, so confirm the borrow works — and sketch what plumbing it would
need. Do not implement it; unit 05 owns that.

### Q6 — What becomes possible that is currently blocked?

RFC-021 and RFC-023 currently emit diagnostics because calamine 0.35 exposes no
content. Report whether `Hyperlink`, merged regions, tables or pivot tables in
0.36 would let either produce structured results instead. Report whether
`mod formats` is still private — if it is, RFC-022 stays blocked and must not be
promised.

Also check `expand_shared_formula`: our current `worksheet_formula` handling may
not expand shared formulas, which would be a silent formula-diff gap
independent of everything else. Report what you find; do not fix it here.

### Q7 — Do the goldens change?

If the spike builds, run the full test suite and report whether any
`expected.json` differs. A calamine upgrade changing our output is a
**significant finding** — it would mean the parser's behaviour changed under us.
Report the exact diff; do not bless anything.

## Required evidence

- Command output for every question above
- `cargo tree` before and after, showing the `quick-xml` version change
- The toolchain matrix result for Q2, naming the lowest toolchain that works
- The full test-suite output from the spike build, including any golden diff

## Deliverable

A written report at
`.git-exclude/review-request/035-handoff-01-calamine-spike/README.md`, in the
package format used throughout M1, answering Q1–Q7 with evidence and ending in a
clear recommendation: **proceed as planned**, **proceed with a different MSRV**,
or **do not proceed**, with reasons.

If the recommendation is anything other than "proceed as planned", say what
decision you need and from whom.

## Prohibited shortcuts

- Do not merge, PR, or push spike code to a branch CI builds.
- Do not answer any question from the vendored source alone. The starting
  hypothesis above came from reading; this unit exists to test it by building.
- Do not bless a golden. If output changes, that is the finding.
- Do not fix anything you discover. Every defect found here is scope for a later
  unit; the value of this one is an accurate map.

## Known risks

- The API delta may be larger than the hypothesis suggests, which would reshape
  unit 02. That is exactly what this unit exists to find out early.
- `zip` 8.6 is a major bump from 7.2 and may carry its own MSRV or behavioural
  surprises. Check it specifically rather than treating it as incidental.

## Review request format

Per development policy §9.2, plus an explicit recommendation as described under
**Deliverable**.
