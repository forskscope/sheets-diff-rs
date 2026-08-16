# Handoffs — RFC-030, fixture coverage (M3 track A)

Companion execution documents for
[RFC-030](../../done/030-extended-fixture-generators-and-corpus-management.md).
Lifecycle state is inherited from that RFC; these do not redefine it.

## Queue

| | Unit | Depends on |
|---|---|---|
| 01 | [Coverage-dimension report](./01-coverage-dimensions-report.md) | — |
| 02 | [Generate the matrix](./02-generate-the-matrix.md) — the nine `rust_xlsxwriter`-producible scenarios | 01 |
| 03 | [The XML-patched scenarios](./03-xml-patched-scenarios.md) — the two that need raw XML | 01 |

Unit 01 is **approved**; RFC-036 was **accepted 2026-08-16**, so 02 and 03 are
live. They are independent of each other and may run in either order.

**Expect a scenario to fail on first run.** Five dimensions in the matrix have
never been asserted, and the last systematic look at this engine produced four
defects. A new assertion failing against current behaviour is the unit working —
it is a finding to report, never something to adjust the assertion around.

## Why this track exists

The planning discussion corrected a diagnosis worth carrying into the work.

It was first claimed that the formula-attachment defect (D-04) hid because our
fixtures are synthetic. That is **false**, and one command disproves it: the
`formula` fixture is `rust_xlsxwriter`-generated as `wb_with_formula(0, 0,
"label", 1, 0, "=1+1")` — a label at row 1, a formula at row 2, which is exactly
the value-range/formula-range origin mismatch D-04 needs. The defect sat in a
synthetic fixture for months.

It hid because **nothing read the golden** until M1, and because **nobody checked
the content when it was first blessed**. Both are now fixed.

The real gap is coverage of **structural patterns**, held ad hoc rather than
systematically. That is generatable — no third-party corpus, no licensing
review, no maintenance burden. **We must not ask ForskScope for customer
workbooks**, and they have been told so.

## Standing constraints

- No change to comparison behaviour. This track adds coverage; if adding
  coverage changes a golden, that is a **finding** — report it.
- Fixtures stay fully synthetic and reproducible: the generator is byte-stable
  and `cargo test` must never write to the corpus.
- Blessing a new golden means **reading the produced JSON and deciding it is
  right**, not observing that the test then passes. See
  `tests/fixtures/corpus/README.md`.
