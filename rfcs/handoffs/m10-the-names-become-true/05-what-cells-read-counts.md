# Handoff 05 — What `cells_read` counts

**Governing.** [RFC-037 §3.5](../../accepted/037-v3-scope.md); RFC-035
(resource safety); RFC-033 (public model lexicon)
**Roadmap.** M10 — unit 05
**Sequence.** **After 01–04 (all landed).** Before the migration guide. Last of
the code units, because it moves every golden.
**Released 2026-09-25**, once the owner settled the question in the next section.

## The decision that shapes this unit

**Settled by the owner 2026-09-25: option (a). Both quantities count populated
cells.** This section records why it needed deciding, because the reason
constrains the work.

`DiffMetrics::cells_read` and `Limits::max_cells_read` are **the same number**.
`src/diff.rs` accumulates one variable:

```rust
*total_cells_read = read_before + area;          // the metric
if let Some(max) = opts.limits.max_cells_read
    && *total_cells_read > max { … }             // the limit, same value
```

So RFC-037 §3.5 — which asks only that the *metric* change — cannot be done
without changing what a documented security preset bounds. `Limits::hardened()`
sets `max_cells_read: Some(5_000_000)`.

**Both change together.** The metric counts populated cells; the bound fires on
that same count. The consequences, which the unit must state rather than
discover:

- A workbook with a **vast bounding box and few populated cells** is no longer
  rejected by `max_cells_read`. Post-streaming it is cheap to read, so this is a
  correction, not a regression — but it is a change to what `hardened()` rejects
  and the threat model must say so plainly.
- A workbook with **many populated cells in a small box** is now bounded, where
  before it was bounded only incidentally.

**The options not taken**, so nobody reopens this: (b) split the two and rename
the limit `max_bounding_box_area`; (c) rename only the metric to
`bounding_box_area`. Both were rejected for the same reason — they keep a bound
on a quantity nothing has spent since the streaming change, which is the defect
this milestone exists to remove.

## Background

`cells_read` reports the **area of the bounding box** of each sheet's populated
cells, summed over sheets and sides — not a count of anything read. On the
`sparse_range` fixture (two populated cells, `A1` and `Z100`), each side's box
is 100 × 26 = 2,600, so it reports **5200** against `cells_compared`'s **2**.

The name says cells. The number is geometry.

**Why it was geometry.** Before f123, `worksheet_range` returned a *dense*
`Range`, and the box area was exactly what it allocated — so bounding the area
bounded the memory. The streaming fix removed that allocation. `src/diff.rs`'s
own doc records the consequence in one clause: *"memory no longer tracks it."*

**So the bound now guards a quantity nothing spends.** A workbook with a vast
box and few cells is cheap to read and is still rejected; a workbook with
millions of populated cells inside a small box is expensive and is bounded only
incidentally. That is the deeper defect, and it is why this unit is not a rename.

## Change scope

- `src/diff.rs` — the accumulator
- `src/model.rs` — `cells_read`'s doc
- `src/options.rs` — `max_cells_read`'s doc, `hardened()`'s doc
- `docs/src/maintainers/threat-model.md` — the *Sheet reading* section
- `docs/src/api-guide.md`, `docs/src/semantics.md` if either asserts on it
- **Every golden under `tests/fixtures/`**
- `rfcs/done/033-*.md`, `rfcs/accepted/035-*.md`
- `CHANGELOG.md`

## Non-change scope

- **Do not change the cancellation poll.** It counts every cell *record*
  streamed, blank or not, and that is correct: blank records cost time. It is a
  different quantity from both candidates here and it stays as it is.
- Do not change `cells_compared`, `diffs_emitted` or `sheets_read`.
- Do not add a new metric field alongside the old one. This is the major; the
  field changes meaning rather than gaining a sibling.
- Do not change `max_input_bytes`, `max_alignment_product`, or any other limit.

## Required implementation

1. **`cells_read` counts populated cells** — the cells the reader actually
   retained, summed over sheets and both sides. A blank record skipped before
   normalisation is not counted, consistent with `hardened()`'s existing
   statement that such records are not counted by `max_cells_read`.
2. **The bound fires on the same quantity**, still inside the streaming loop,
   still before the cell is retained, so it fires before the memory is spent.
3. **`cells_read`'s doc is rewritten** and must state what it now counts, that
   it is **not** the bounding-box area, and keep a worked example. Keep the
   `sparse_range` contrast — the numbers invert, and showing the new figure
   beside `cells_compared` is what stops the next reader assuming geometry.
4. **`max_cells_read`'s doc says what it bounds** and that it bounds the memory
   the cell map costs, which the box area did not after the streaming change.
5. **`hardened()`'s doc is re-examined against its new meaning.** Its
   `# What it bounds` / `# What it does not bound` sections were written when
   this field meant box area. **At least one of those sentences is now wrong** —
   find it. The blank-record clause is likely still right; check rather than
   assume.
6. **The threat model's *Sheet reading* section** records that the bound now
   tracks retained cells, and — this is the part to get right — **whether the
   sparse-box workbook that motivated f123 is still rejected, and if not, why
   that is acceptable.** If it is no longer rejected, say so plainly; a threat
   model that quietly drops a case is worse than one that admits it.
7. **Re-bless every golden**, and show each moved file differs **only** in
   `cells_read`.

## Required tests

- `sparse_range`: `cells_read` is the populated-cell count, and the test states
  both numbers so the inversion is visible in the source.
- A dense fixture where box area and populated count coincide: `cells_read` is
  unchanged from 2.6.0. **This is the control** — without it, a mistake that
  zeroes the metric passes everything else.
- `max_cells_read` fires on populated cells: a sheet with a large box and few
  cells is **accepted** under a bound that the old area would have exceeded; a
  sheet with many cells in a small box is **rejected**. Both directions, because
  this is the pair that changes.
- The bound still fires **mid-sheet**, before the whole sheet is retained.
- `cells_read >= cells_compared` still holds, if that invariant survives —
  **check it rather than assuming; it was true of the area and may not be true
  of the count.** If it does not hold, that is a finding and the doc must stop
  claiming it.

## Acceptance criteria

1. `cells_read` counts populated cells, cumulative over sheets and sides.
2. The bound fires on the same quantity, mid-sheet, before retention.
3. Docs rewritten: the metric, the limit, `hardened()`, the threat model.
4. **The wrong sentence in `hardened()` is found and named in the review
   request** — not merely fixed.
5. Every golden re-blessed, each shown to differ only in `cells_read`, with a
   count of how many moved.
6. The dense-fixture control passes.
7. Both directions of the limit change are tested.
8. The `>= cells_compared` invariant is verified or reported as broken.
9. CHANGELOG `### Changed`, naming the metric, the limit, and that
   `hardened()`'s effective strictness changes in both directions.
10. Gates green.

## Prohibited shortcuts

- **Do not keep the area as a second, hidden accumulator** so the limit behaves
  as before. One number, one meaning.
- Do not re-bless goldens without reading the diff. Every prior golden move in
  this project was verified line by line; this one moves the most files and is
  the easiest place to hide a second change.
- Do not "fix" the invariant by clamping. If `cells_read < cells_compared` is
  now possible, that is a fact about the two definitions.
- Do not adjust `hardened()`'s **value** to compensate. Whether 5,000,000 is
  still the right number is a separate question; changing it here would bury a
  policy decision inside a semantic one.

## Compatibility constraints

**Two breaks, one of them behavioural and security-relevant.**

1. `DiffMetrics::cells_read` reports a different number. No type changes, so
   Rust callers compile; anyone who stored or asserted on it sees a change.
   `--format json` publishes it, so this is a machine-readable surface.
2. **`max_cells_read` rejects a different set of workbooks.** Some that were
   rejected are now accepted and some that passed are now rejected. That is the
   point — the bound starts tracking a real cost — and it must be stated as a
   behaviour change, not as a fix.

## Known risks

- **Every golden moves.** That is expected and it is also cover for an
  accidental second change. Criterion 5 exists for that reason.
- `docs/src/` executes; anything asserting `cells_read` moves.
- **`hardened()` is cited by the threat model and by the ForskScope
  correspondence.** Its meaning changing is the kind of thing that leaves a true
  sentence false elsewhere; sweep for citations rather than only editing the
  function's own doc.

## Required evidence

- Before/after `cells_read` for every corpus scenario, as a table
- The count of goldens moved and proof each differs only in `cells_read`
- The dense-fixture control
- Both directions of the limit change
- The `>= cells_compared` check, whichever way it comes out
- The `hardened()` sentence you found wrong, quoted
- Gates, each with exit status
- CI run link

## Review request format

Per development policy §9.2. Additionally: quote the `hardened()` sentence that
this unit made false, and state whether the f123 sparse-box workbook is still
rejected.
