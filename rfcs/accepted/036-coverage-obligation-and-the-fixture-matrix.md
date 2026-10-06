# RFC-036 — Coverage Obligation and the Fixture Matrix

**Status.** Accepted (2026-08-16) — and **deliberately still `accepted/` rather than `done/`**, ruled M9 unit 03 (2026-10-01): §5.3's coverage obligation is a standing rule with no finished state, and §5.4 gained a new normative sentence as recently as 2026-10-01, which is not something that happens to a historical record. The original eleven-scenario matrix shipped at M3; rows 12–14 were added by M9 unit 05. (The previous wording here — "track A units 02–03 are live" — referred to a tracking scheme that predates M4 and was stale on any reading.)
**Target:** M3
**Created:** 2026-08-16
**Extends:** RFC-015, RFC-030
**Related:** RFC-034 (the golden corpus this builds on), G-009

**Note added M10 unit 06 (3.0.0):** this RFC names `AlignmentMode::HeaderColumn` as an uncovered mode (§ evidence and
matrix row 4). That mode was removed in 3.0.0 — it was `RowKey { columns: vec![1] }` under another name, and the
corpus scenario `alignment_header_column` now covers that alignment. The matrix row stands as history.

**Note added M9 unit 05 (2026-10-01) — a gap this RFC never named, not an obligation it stated and missed.**
§5.1's definition is about *behaviour for a dimension*; it never says every `DiagnosticKind` the engine can
construct must be producible by some scenario. Checked directly: `MissingAlignmentKey`, `AlignmentBoundExceeded`
and `FormulaUnavailable` are absent from both this RFC's §5.2 matrix and `tests/fixtures/corpus/README.md`'s copy
of it, and the corpus (`compare_bytes` on every `tests/fixtures/generated/*` pair, default options) produced none
of them before this unit. **Not a violation of an existing obligation — the obligation was never written.**
Matrix rows 12–14 close all three: 12 (`formula_unavailable`) through the ordinary default-options path; 13
(`missing_alignment_key`) and 14 (`alignment_bound_exceeded`) through the same dedicated-non-default-options
escape hatch rows 3/4 already established for `AlignmentMode` coverage — a committed fixture, picked up by the
default-options golden like any other, plus a dedicated assertion in `tests/integration.rs` that applies the
mode or bound the scenario exists to cover. **Proposed sentence for §5.4, not yet ruled on** (this unit's
handoff: *"propose the one sentence... and let me rule"*): *"Every `DiagnosticKind` variant the engine can
construct must be reachable by at least one scenario's assertion — through the default-options corpus path
where possible, or the dedicated-options pattern rows 3/4 established otherwise — or be listed here as
explicitly deferred with a reason."* Found four more codes with the same gap while checking this
(`ambiguous_sheet_match`, `unsupported_workbook_metadata`, `defined_name_scope_unknown`,
`duplicate_alignment_key`) — out of this unit's scope, reported in the review request, not fixed here.

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

> **Sharpened 2026-09-27, from an outside reader.** *"The behaviour for that
> dimension"* is ambiguous when a dimension has **more than one** behaviour worth
> protecting, and then "covered" is itself a marker credited with more than it
> measures. **A dimension's matrix row must say which behaviour its assertion
> protects** — correctness, resource cost, or both — because an assertion on one
> would not fail if the other broke.
>
> The case that produced this: **row-count asymmetry**. In our aligned path it is
> a **resource** dimension — an asymmetric pair is the only shape that reaches
> the coordinate-set ceiling (f131), and a symmetric fixture of the same size
> never will. ForskScope checked the same dimension in their line diff, expecting
> to find the same gap, and **measured every asymmetric shape as *cheaper* than
> the symmetric pair of the same size** (40,000 lines: 52.7 ms symmetric against
> 15.7–32.6 ms for four asymmetric shapes) — cost tracks total input, so there is
> no ceiling to guard. There an asymmetric fixture is a **correctness**
> dimension: it buys hunk boundaries and one-sided rendering.
>
> Same word, different job, and their corpus *"would have read as covered by a
> reviewer who did not ask that question."* So would ours.

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
- **Every `DiagnosticKind` variant the engine can construct must have a scenario
  whose assertion fires on that diagnostic** — through the default-options corpus
  path where the diagnostic's precondition is reachable there, or through the
  dedicated-options pattern rows 3/4 established otherwise — **or be listed here
  as explicitly deferred, with a reason.** The assertion must be on the
  diagnostic itself, per §5.1: a scenario that merely *produces* it while
  asserting something else does not cover it, and a scenario that produces it by
  accident covers nothing (two goldens did exactly that for
  `formula_unavailable` until f135 removed the accident).

  *Extended 2026-10-06 (confidence-that-measures-counts/02):* `DuplicateRowSignature` is
  covered by corpus row 19 through its dedicated-options pattern, with the assertion on the payload.

  *Adopted 2026-10-01, from M9 unit 05's proposal, with §5.1's "assert on the
  thing" requirement made explicit. The set is knowable:
  `every_code_in_the_table_is_producible_and_nothing_else_is` already enumerates
  it. **Deferred at adoption, pending M9 unit 07:** `ambiguous_sheet_match`,
  `unsupported_workbook_metadata`, `defined_name_scope_unknown`,
  `duplicate_alignment_key` — found by unit 05 while closing rows 12–14, and
  deferred rather than silently absent, which is what the second bullet above
  asks for.*

  *Closed, M9 unit 07 (2026-10-01) — all four, none genuinely unreachable. Each
  was a shape question, not an options one, once checked rather than assumed:
  `ambiguous_sheet_match` (matrix row 15) needs two unmatched sheets on each
  side under the default mode; `duplicate_alignment_key` (row 16) needs the
  same dedicated-`RowKey` pattern as rows 3/4/13/14; `unsupported_workbook_metadata`
  and `defined_name_scope_unknown` (row 17, one scenario closes both — the
  metadata pass that raises them runs unconditionally, per `meta.rs`'s own doc
  comment) need only a workbook with one defined name, changed between old and
  new. `ambiguous_sheet_match` was checked against RFC-009 §8 before a fixture
  was written, per *Required implementation* 4 of the unit's handoff: the
  engine reports **both** "leave as add/remove" and an ambiguity warning
  together, which satisfies and exceeds §8's "either/or" — not a defect.
  (The unit's own handoff named `SheetChange::Ambiguous` as the expected
  mechanism; no such variant exists in this crate's model, and none is
  needed — §8 asks for add/remove-plus-warning, which is what the two
  existing mechanisms, `SheetChange::{Added,Removed}` and
  `DiagnosticKind::AmbiguousSheetMatch`, already give.)*

### 5.5 Options: every constructor, not every value

**Added 2026-10-06.** Owed since 2026-10-05 and twice mis-stated before being written; the history is
part of the rule, because both failed attempts failed in instructive ways.

> **For every public option, the matrix must name each **constructor** the option admits — every enum
> variant, and both arms of every `Option` — and each one must be constructed by some test. An option
> whose constructors are not enumerated is not covered, whatever the scenario count says.**

#### The defect this comes from

`AlignmentMode::RowSignature { sample_columns: Option<Vec<u32>> }` shipped with `RowSignature` and was
never fuzzed, never asserted, and — as it turns out — never fully constructed. Under
`sample_columns: Some(cols)`, a row with no cell in any sampled column got no entry in the signature
map, so it was in none of `matched`, `removed` or `inserted`, and **its cells were never compared**: a
real change produced zero cell diffs, zero diagnostics, and an `alignment_summary` reporting `Exact`.
It was f130's defect on the path f130 did not touch, and it survived every release until 3.4.0.

#### Why "every mode constructed by a test" was too weak

That was the first wording, and it is wrong because **the mode was constructed.** Measured at tag
3.3.0: seven tests construct `AlignmentMode::RowSignature`, and **every one of them passes
`sample_columns: None`.** Not one passed `Some(...)`. The mode was covered by any count you like; the
defect lived in an argument to it.

The lesson is that an option's *own* fields have variants too, and a rule that stops at the outer
enum stops one level above where the inputs actually branch.

#### Why "every option value" is impossible

That was the second wording. `sample_columns` is a `Vec<u32>`: the value space is unbounded, so the
rule cannot be satisfied and therefore cannot be a rule. This is why the obligation lands on
**constructors**, which are finite and readable off the type — `None` and `Some` here; four variants
for `AlignmentMode`; and so on — rather than on values, which are not.

#### What I got wrong about the instrument, and the correction

I was going to write that a coverage tool would not have caught this, on the reasoning that the gap
was a missing *case* rather than a missing *line*. **That is false, and checking it is what produced
this section.** With no test passing `Some(...)`, the `if let Some(cols) = sample_cols` filter in
`compute_row_signatures` was **never executed**, so ordinary branch coverage would have reported it
uncovered for the whole life of the mode.

So this rule is not a substitute for an instrument we lack; it is a written-down form of one we were
not using. **Two consequences:**

1. The enumeration is cheap and belongs in the matrix regardless, because it is reviewable by reading
   a type and a test list, with no tooling.
2. **Branch coverage over the test suite is worth adding**, and would have found this without anyone
   enumerating anything. Not scoped here — it is a CI-cost decision for the owner — but recorded, so
   that the next person weighing it knows it had a concrete catch.

#### What this does not cover

A constructor that is built by a test **but only in its easy shape**. `Some(cols)` where every row has
a sampled cell exercises the constructor and misses the defect. The constructor list is a floor, not a
ceiling, and §5.1's definition still governs: an assertion that would fail if the behaviour broke.
Enumerating constructors makes the floor checkable; it does not make the matrix complete.

#### Relationship to the row-alignment invariant

`docs/src/maintainers/row-alignment-invariant.md` catches the whole class differently and without
enumeration: a row that disappears violates it under *any* mode and *any* option value, so the
assertion fires in the first test that happens to reach the shape. **Prefer an invariant where one
exists.** This section is for where none does.

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
