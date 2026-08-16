# Handoff 02 — Generate the matrix

**Governing RFC.** [RFC-036](../../accepted/036-coverage-obligation-and-the-fixture-matrix.md) §5.2; RFC-030 for mechanism
**Roadmap.** M3, track A
**Sequence.** After unit 01 (approved). Unit 03 may run in parallel or after.

## Purpose

Build the nine `rust_xlsxwriter`-producible scenarios of RFC-036's matrix, each
covered in the sense §5.1 defines: **an assertion that would fail if the
behaviour broke.**

## Change scope

`examples/gen-fixtures.rs`, `tests/fixtures/generated/*/` (nine new scenarios),
`tests/integration.rs`, `tests/fixtures/corpus/README.md`, `CHANGELOG.md`.

## Non-change scope

Do **not** touch `src/`. Do not modify the seven existing scenarios — they stay
byte-identical. Do not attempt scenarios 10 or 11; those are unit 03. Do not
attempt the `<dimension>`-tag case: unit 01 established it is not a hazard,
because calamine uses the tag only as a capacity hint and derives range bounds
from the collected cells.

## Required implementation

Scenarios 1–9 from RFC-036 §5.2. For each: a fixture pair, a golden, **and an
explicit assertion on the property the scenario exists to protect.**

A golden alone does not satisfy §5.1 where the correct answer is knowable
independently. For scenario 1 that means asserting the reported address is the
one you wrote to — not merely that the output matches a blessed file. The
`formula` fixture had a golden for a year and the golden was wrong.

Notes per scenario:

- **#2's negative control** — the companion where the *first* row is the formula
  — matters as much as the positive case. It is the shape that would have hidden
  D-04 entirely, and it guards against a future "fix" that special-cases
  coinciding origins instead of translating through absolute coordinates.
- **#5** uses `Formula::new("1/0").set_result("#DIV/0!")`; unit 01 confirmed
  `rust_xlsxwriter` pattern-matches the error literals and writes `t="e"`.
  Assert on `CellError` variants and on `ValueDifferenceKind::ErrorKindChanged`,
  which no test in this crate has ever asserted.
- **#6 carries a known unknown.** It needs sheet *reordering*, which unit 01
  probed for chart sheets but **not** for reordering. **Probe it first.** If
  `rust_xlsxwriter` cannot express it, stop and report with a proposed
  alternative — do not substitute a different scenario silently (RFC-036 §6).
  The scenario must make `SheetChange::Moved` actually occur and be asserted
  distinctly from `Unchanged`; today's only test accepts either.
- **#3 and #4** are about `AlignmentMode`, not workbook content. Existing
  generator output can drive them; what is new is the assertion.

## Required tests

One assertion per scenario, minimum, on the property named in RFC-036 §5.2's
"Closes" column. Where a scenario closes two gaps (#2, #6), assert both.

## Required documentation

`tests/fixtures/corpus/README.md` gains the matrix and RFC-036 §5.3's
obligation, per §5.4 — the matrix must be visible where fixtures are written,
not only in the RFC.

## Acceptance criteria

1. Nine scenarios exist, each with a fixture pair, a golden, and an assertion
   satisfying §5.1.
2. `cargo test` leaves the corpus untouched; generation stays byte-reproducible.
3. The seven pre-existing scenarios are byte-identical — verify the corpus hash
   for those directories separately from the new ones.
4. `SheetChange::Moved` is asserted distinctly from `Unchanged`, or #6's
   inexpressibility is reported with an alternative.
5. `ErrorKindChanged` is asserted.
6. The matrix and the §5.3 obligation are in the corpus guide.
7. Full matrix, `fmt`, `clippy -D warnings`, `deny`, MSRV, CI green.
8. **No comparison behaviour changes.** If a new assertion fails against current
   behaviour, that is a **defect the coverage caught** — stop and report it.
   Do not adjust the assertion to pass.

## Prohibited shortcuts

- Do not satisfy §5.1 with a golden where the correct answer is independently
  knowable. That is the exact failure this RFC exists to prevent.
- Do not bless a new golden without reading it and deciding it is right.
- Do not weaken a scenario because it is awkward to generate. Report instead.
- Do not fix any defect found. Report it for M4's queue.

## Known risks

- **Expect at least one scenario to fail on first run.** Five dimensions have
  never been asserted; the last systematic look at this engine produced four
  defects. A failure here is the unit working, not a setback.
- #6's reordering may be inexpressible. See above.

## Required evidence

- Per scenario: the assertion, and its output
- The corpus hash for the seven pre-existing scenarios, unchanged
- `git status` after a full test run, clean
- CI run link
- Any defect found, with enough detail to schedule

## Review request format

Per development policy §9.2, plus an explicit statement of whether any new
assertion failed against current behaviour, and what it revealed.
