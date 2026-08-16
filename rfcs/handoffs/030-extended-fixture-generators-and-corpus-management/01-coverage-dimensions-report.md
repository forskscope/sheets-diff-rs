# Handoff 01 — Coverage-dimension report

**Governing RFC.** [RFC-030](../../done/030-extended-fixture-generators-and-corpus-management.md); G-009 of the v2 requirements
**Roadmap.** M3, track A
**Sequence.** First unit of M3.

## Purpose

Enumerate the **structural dimensions of a workbook that can produce a wrong
answer**, and report which are currently covered, which are not, and which
cannot be produced with `rust_xlsxwriter`.

**This unit lands no fixtures.** Its deliverable is a report and a proposed
matrix. Units 02 and 03 are written from it.

## Background

G-009 requires *"a broad test corpus covering the integration failures and
common spreadsheet edge cases."* The corpus covers seven scenarios chosen for
particular features. Nothing enumerates the structural space, so coverage is
whatever past work happened to need.

That is how D-04 survived. The `formula` fixture — `wb_with_formula(0, 0,
"label", 1, 0, "=1+1")` — contains the exact pattern that triggers it: a
non-formula cell above a formula, so the formula range's origin is later than
the value range's. The pattern was present; nothing asserted the address was
right, and the first bless froze the wrong answer.

Note what this means for the report: **a dimension can be present in a fixture
and still uncovered**, because coverage is an assertion, not a file. Say which
dimensions are *asserted on*, not merely which appear somewhere.

## Change scope

The report only. Optionally throwaway probes to answer a question — not
committed.

## Non-change scope

Do not add fixtures, change comparison behaviour, touch the corpus, or modify
`src/`. Do not obtain third-party workbooks; **do not ask ForskScope for
customer files.**

## What to enumerate

At minimum, and say for each: what could go wrong, whether any current fixture
exercises it, and whether anything *asserts* on it.

**Range geometry** — origin not at A1; leading empty rows and/or columns; a
single populated cell; one row; one column; sparse interior; a used range whose
declared dimension disagrees with the populated cells.

**Parallel-range attribution** — the D-04 class. Value range and formula range
with different origins; a sheet where only some cells carry formulas; a sheet
where the *first* cell is a formula (the case that would have masked D-04).
Consider whether other paired ranges exist with the same hazard.

**Sheet-level shape** — zero sheets; one; many; a sheet with no populated cells
beside a populated one; sheets whose order differs between the pair; a
non-worksheet sheet type beside a worksheet.

**Cell-content shape** — each `CellValue` variant reachable through `.xlsx`;
empty string versus absent cell; a cell whose value is present but whose formula
is not, and the reverse.

**Alignment inputs** — since alignment reads a separate coordinate space: rows
that collide numerically between old and new (D-03's class); duplicate keys;
a sheet at and just over the bound.

Add dimensions you find. The list is a floor.

## Required output

`.git-exclude/review-request/030-handoff-01-coverage-dimensions/README.md`,
containing:

1. **A dimension table.** Dimension → what could go wrong → fixture that
   exercises it (or none) → assertion that checks it (or none).
2. **A gap list**, ordered by *consequence* — a gap that could produce a silent
   wrong answer outranks one that could produce a loud error. Say why for each.
3. **A generatability split.** Which gaps `rust_xlsxwriter` can produce (unit
   02) and which need XML patching (unit 03). `tests/support.rs::patch_xlsx_xml`
   exists and five tests use it.
4. **A proposed matrix**, with a size estimate. If it is large, say what a
   defensible subset is and on what principle — a matrix nobody maintains is
   worse than a small one that is kept true.
5. **A recommendation on whether this warrants an RFC.** If the matrix
   introduces a design decision beyond RFC-030's scope — how coverage is
   defined, how the matrix is maintained, what "covered" obliges — say so and it
   becomes RFC-036. If it is execution within RFC-030, say that instead. Do not
   open one yourself.

## Prohibited shortcuts

- Do not report a dimension as covered because a fixture happens to contain it.
  Coverage means an assertion would fail if the behaviour broke.
- Do not propose a matrix so large it will not be maintained.
- Do not fix anything found. A defect discovered here is a finding for M4's
  queue, reported with enough detail to schedule.
- Do not guess at whether `rust_xlsxwriter` can emit something. Try it.

## Known risks

- The enumeration may be larger than the corpus can absorb. That is a real
  result — report it with a subset proposal rather than trimming silently.
- You may find defects. Expect it: the last systematic look at this engine
  produced four. Report, do not fix.

## Required evidence

- The dimension table, with per-row evidence for "asserted on" claims
- Command output for any generatability question answered by trying it
- The gap list with consequence ordering and reasoning

## Review request format

Per development policy §9.2, plus the RFC recommendation from output item 5.
