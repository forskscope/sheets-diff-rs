# Handoffs — RFC-030, fixture coverage (M3 track A)

Companion execution documents for
[RFC-030](../../done/030-extended-fixture-generators-and-corpus-management.md).
Lifecycle state is inherited from that RFC; these do not redefine it.

## Queue

| | Unit | Depends on |
|---|---|---|
| 01 | [Coverage-dimension report](./01-coverage-dimensions-report.md) | — |
| 02 | Generate the matrix and wire it into CI | 01 |
| 03 | Extend `patch_xlsx_xml` to what `rust_xlsxwriter` cannot emit | 01 |

Units 02 and 03 are **not written yet**, deliberately: their scope is whatever
01 finds uncovered. Writing them now would mean guessing at the matrix before
the dimensions are enumerated — the same reason M2's unit 02 waited on its spike.

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
