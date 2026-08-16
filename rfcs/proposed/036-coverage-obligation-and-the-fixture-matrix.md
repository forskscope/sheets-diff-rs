# RFC-036 — Coverage Obligation and the Fixture Matrix

**Status.** Proposed
**Target:** M3
**Created:** 2026-08-16
**Extends:** RFC-015, RFC-030
**Related:** RFC-034 (the golden corpus this builds on), G-009

## 1. Summary

Define what **covered** means for this project, fix the initial set of
structural scenarios the fixture corpus must contain, and state the standing
obligation that keeps it true as the engine changes.

RFC-030 defines the fixture *mechanism* — generator design, on-disk layout,
scenario metadata — and explicitly disclaims completeness as a non-goal. This
RFC supplies what that leaves open: not how fixtures are produced, but which
must exist and why, and what a future change is required to bring with it.

## 2. Motivation

G-009 requires *"a broad test corpus covering the integration failures and
common spreadsheet edge cases."* The corpus has seven scenarios, each added for
a particular past feature. Nothing enumerates the structural space, so coverage
is whatever previous work happened to need.

That is not a hypothetical weakness. It is how D-04 survived.

The `formula` fixture — `wb_with_formula(0, 0, "label", 1, 0, "=1+1")` —
contains the exact pattern that triggers the formula-attachment defect: a
non-formula cell above a formula, so the formula range's origin is later than
the value range's. **The pattern was present from the day the fixture was
written.** The defect survived because nothing asserted the reported address was
right, and the first bless froze the wrong answer into the golden.

The distinction that matters is therefore not *"is there a fixture?"* but
*"would an assertion fail if this broke?"* — and the coverage report found that
distinction separates several dimensions this project believed it covered:

- `SheetChange::Moved` is produced by `matcher.rs` and its only test asserts
  `Unchanged | Moved`, accepting either. It cannot distinguish them.
- `CellError` comparison has **zero** coverage at any level; `ErrorKindChanged`
  is asserted by nothing.
- Two of the three alignment modes — `RowSignature`, `HeaderColumn` — have
  never been exercised by any test.
- No golden-corpus fixture uses dates at all, despite dates being where four
  M2 defects lived.

Without a stated obligation, this recurs. A corpus that grows only when someone
happens to need a fixture drifts back to exactly the state that hid D-04.

## 3. Goals

- Define **covered** as an assertion property, not a file property.
- Fix an initial scenario set that closes the highest-consequence gaps.
- State the obligation that keeps the matrix true as the engine changes.
- Keep the matrix small enough to be maintained.

## 4. Non-goals

- **Not** a cross-product of every dimension against every other. Most
  combinations do not interact, and a matrix nobody maintains is worse than a
  small one that is kept true.
- **Not** testing every OpenXML feature — RFC-030's non-goal, unchanged.
- **Not** a real-world or third-party workbook corpus. The gap is structural
  patterns, which are generatable; a licensed corpus would add licensing review,
  maintenance and reproducibility costs and buy less. **Customer workbooks must
  never be requested from consumers.**
- **Not** a fix for unreachable model variants. `CellValue::Integer`,
  `Duration` and `Unsupported` cannot occur through `.xlsx` at all; a fixture
  cannot close an architectural unreachability. That is a separate design
  question (§8).

## 5. Design

### 5.1 The definition

> A dimension is **covered** when an assertion exists that would fail if the
> behaviour for that dimension broke.

A fixture that contains a pattern is not coverage. A golden that records
behaviour is coverage **only for change** — it cannot detect having been born
wrong, which is precisely what happened to `formula`. Where a dimension's
correct answer is knowable independently, coverage means an explicit assertion
on that answer, not only a golden.

### 5.2 The initial matrix

Eleven scenarios, chosen to close every gap ranked 1–5 by consequence in the
coverage report, folding closely-related gaps together where they compose
naturally:

| # | Scenario | Closes |
|---|---|---|
| 1 | Data block starting at row 5+, value-only | origin not at A1, row axis |
| 2 | As #1 with a formula whose origin also isn't row 1; plus a companion where the first row *is* the formula | origin row axis; the D-04 negative control |
| 3 | `RowSignature` alignment over an insert/delete shape | alignment mode coverage |
| 4 | `HeaderColumn` alignment over header-plus-data | alignment mode coverage |
| 5 | Two differing error kinds, plus an unchanged-error pair | `CellError`, `ErrorKindChanged` |
| 6 | Three sheets: one unchanged, one changed, one whose index differs | `SheetChange::Moved`; many-sheets in corpus |
| 7 | A serial-based date column, one changed | dates in the corpus |
| 8 | Non-ASCII sheet name and cell text | text encoding |
| 9 | A chart sheet beside a worksheet | non-worksheet sheet types |
| 10 | A physically-present empty cell before real content | calamine's empty-cell range-anchoring behaviour |
| 11 | ISO `DateTimeIso` promoted from a hand-built test into the corpus | ISO dates in the corpus |

Scenarios 1–9 are producible with `rust_xlsxwriter`; 10 and 11 need
`patch_xlsx_xml`. Both facts were established by probe, not assumption.

Ordering is by consequence: a gap that could produce a **silent wrong answer**
outranks one that could only produce a loud error, because silent wrong answers
are the failure class the 2.3.0 release existed to close.

### 5.3 The obligation

**A change to `normalize.rs`, `compare.rs`, `align.rs`, or `diff.rs` that alters
behaviour for a dimension in the matrix must arrive with an assertion for that
dimension, or state in its review request why none is needed.**

This is deliberately a review-time obligation rather than an automated gate.
Automating "did this change need a fixture?" is not tractable; making it a
question the reviewer must see answered is. The failure it prevents is the one
this project has already had twice — a code path acquiring behaviour that
nothing checks.

### 5.4 Keeping the matrix true

- New dimensions are added to the matrix when found, not deferred to a future
  audit. The coverage report is a snapshot; this RFC is the living record.
- A dimension may be **explicitly deferred** with a stated reason. An
  undocumented gap is a defect; a documented one is a decision.
- The matrix lives in `tests/fixtures/corpus/README.md` alongside the
  contribution guidance, not only in this RFC, so it is visible where fixtures
  are written.

## 6. Testing and verification

This RFC *is* test policy; its verification is that the eleven scenarios exist,
each with an assertion satisfying §5.1, and that the corpus guide carries the
matrix.

One scenario needs a caveat: **#6 depends on `rust_xlsxwriter` being able to
express sheet reordering**, which the coverage report flagged as probed for the
chart-sheet case but *not* for reordering. If it turns out inexpressible, say so
and propose an alternative rather than substituting one silently.

## 7. Alternatives considered

- **Execution within RFC-030.** Rejected: RFC-030 disclaims completeness, and
  §5.3's obligation is new policy, not mechanism.
- **A full dimensional cross-product.** Rejected as unmaintainable; §4.
- **An automated coverage gate.** Rejected as intractable; §5.3.
- **A real-world corpus.** Rejected; §4 and the provenance analysis that
  showed the gap was never authorship.

## 8. Deferred to a separate decision

`CellValue::Integer`, `Duration` and `Unsupported` cannot be produced by any
`.xlsx` input, and `FormatChange`, `CellNumberFormat`, `WorkbookChange` and
`WorkbookObjectChange` are permanently empty. Three unreachable variants and
four inert types is a public-model question — whether they stay, documented as
reserved, or the model shrinks to what the engine delivers, which would be
breaking.

**Out of scope here.** Recorded so it is not mistaken for a coverage gap that
fixtures could close.

## 9. Acceptance criteria

1. The eleven scenarios exist, each satisfying §5.1's definition of covered.
2. Each is generated reproducibly; `cargo test` never writes to the corpus.
3. The matrix and §5.3's obligation appear in `tests/fixtures/corpus/README.md`.
4. Any scenario that proves inexpressible is reported with an alternative, not
   silently replaced.
5. No comparison behaviour changes. If adding coverage moves a golden, that is a
   **finding** — a defect the new assertion caught — and is reported, not
   blessed away.
