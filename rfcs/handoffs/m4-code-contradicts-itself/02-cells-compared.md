# Handoff 02 — `DiffMetrics.cells_compared`

**Governing RFCs.** RFC-024 (large-workbook memory), RFC-027 (benchmark governance)
**Roadmap.** M4
**Sequence.** Any.

## Purpose

Make `cells_compared` count what its name and its changelog entry both claim it
counts.

## Background

`DiffMetrics.cells_compared` is a public field. It currently equals
`cells_changed`, always.

The accumulator in `src/diff.rs` reads

```rust
metrics.cells_compared += sheet_diff.summary.cells_changed as u64
    + sheet_diff.cell_diffs.iter()
        .filter(|cd| cd.value.is_none() && cd.formula.is_none())
        .count() as u64;
```

`build_sheet_diff` skips any coordinate where both facets are `None`, so the
second term is **always zero**.

CHANGELOG 2.2.3 states: *"`DiffMetrics.cells_compared` now counts all coordinate
pairs visited, not just changed cells."* It did not then and does not now. That
entry was annotated as wrong in M2 unit 06; this unit makes the annotation
unnecessary.

## Change scope

`src/diff.rs`, `src/model.rs` (the field's doc comment if it needs correcting),
`tests/integration.rs`, `CHANGELOG.md`.

## Non-change scope

Do not change comparison behaviour, the diff output, or any other `DiffMetrics`
field. `cells_read` and `diffs_emitted` are correct — confirm rather than assume,
and report if either is not.

## Required implementation

1. **Decide what "compared" means, and say so.** The coordinate set built in
   `build_sheet_diff` is the union of both sides' populated cells, possibly
   remapped by alignment. Every coordinate in it is compared, whether or not it
   produces a diff. That is the natural reading and almost certainly the intent —
   but state it explicitly in the field's doc comment, because the ambiguity is
   how the field came to be wrong.
2. **Count it where the comparison happens**, not by inferring it afterwards
   from the results. The current formula is wrong precisely because it tries to
   reconstruct the count from `cell_diffs`, which by construction only holds
   coordinates that produced a diff.
3. **Check the parallel-path lesson does not recur.** M2 removed a code path
   whose metrics accounting diverged from the sequential one. There is one path
   now; keep it that way.

## Required tests

An assertion that `cells_compared` exceeds `cells_changed` for a fixture where
most cells are unchanged. `sparse_range` and `row_insertion_cascade` both have
that shape. A test that only checks `cells_compared > 0` would pass today and is
not sufficient.

Per RFC-036 §5.3, this behaviour change arrives with its assertion.

## Acceptance criteria

1. `cells_compared` counts coordinates compared, not diffs emitted.
2. Its doc comment states what it counts unambiguously.
3. A test would fail if it reverted to equalling `cells_changed`.
4. The dead `filter(...)` term is gone.
5. **The fixture corpus moves**, because `metrics` is serialised into every
   golden. That is expected here. Every changed golden must be shown to differ
   **only** in `cells_compared`, and the review request must say so explicitly.
6. CHANGELOG records the correction and notes that 2.2.3's claim is now true.
7. Gates green.

## Prohibited shortcuts

- Do not make the test assert an exact number unless it is stable across
  platforms and feature sets. A relational assertion (`compared > changed`) is
  more robust and tests the property that actually matters.
- Do not bless the moved goldens without reading them. If a golden differs in
  anything besides `cells_compared`, stop — that is a finding.
- Do not "fix" the metric by redefining it as `cells_changed` in the docs. The
  field's name and its changelog both promise otherwise.

## Known risks

Every golden containing a non-zero metric will move. That is a large diff for a
small fix, and the risk is that a real change hides in it. Criterion 5 exists for
that reason — verify field-by-field, not by eye.

## Required evidence

- The diff
- The golden diff, shown to touch only `cells_compared`
- The new assertion and its output
- Full matrix, gates, CI green

## Review request format

Per development policy §9.2, plus explicit confirmation that the golden diff
contains nothing but `cells_compared` changes.
