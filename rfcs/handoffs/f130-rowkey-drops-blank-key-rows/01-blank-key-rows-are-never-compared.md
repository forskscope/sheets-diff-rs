# Handoff f130-01 — `RowKey` drops rows with a blank key, and calls it Exact

**Governing.** RFC-011 (row alignment); ROADMAP §6 (a missed difference is an
integrity failure)
**Sequence.** **Ahead of M9.** This is a defect response, not a milestone unit.
**Release:** 3.0.1, patch.

## Purpose

With `AlignmentMode::RowKey`, a row that has no cell in the key column is
excluded from alignment and **never compared**. A change in such a row is
reported as no change at all.

## Background

Reported by ForskScope on 2026-09-26, from measuring each alignment mode's
*failure* modes rather than its success cases. **Reproduced here**, and it is
worse than they reported.

`extract_row_keys` (`src/align.rs`) builds its map by iterating cells and
inserting `rows.entry(*r)` only for rows that **have a cell in a key column**.
A row whose key cell is blank never enters the map. `row_key_alignment` then
calls `lcs_match(old_keys, new_keys)`, so that row is in none of `matched`,
`removed` or `inserted` — it is not compared, and nothing says so.

Four rows, one with a blank key, one cell changed in that row:

```
Positional            cell_diffs = 1     (the truth)
RowKey { columns:[1] } cell_diffs = 0
                       alignment_summary = matched_rows: 3, inserted: 0,
                                           removed: 0, confidence: Exact
```

**The result does not merely omit the difference — it affirms a clean
alignment.** `confidence: Exact` with no diagnostic is a positive claim that the
rows were matched exactly, made about rows that were not looked at.

ForskScope's own shape: 2,000 rows with a unique `ID` blank in ~5% of them —
subtotal lines, spacers, notes. 1,901 of 2,001 rows matched; the other 100
silently absent. That is how real sheets look, so this is not a corner case.

**This is the reorder defect's class**: the engine has the information, the
answer says there is no difference, and nothing objects.

## Change scope

- `src/align.rs`
- `src/model.rs` — only if `AlignmentSummary` gains a field (see item 3)
- `tests/`
- `rfcs/done/011-*.md`
- `CHANGELOG.md`

## Non-change scope

- **Do not change `Positional` or `RowSignature`.** Neither has this defect;
  ForskScope measured both.
- Do not change what `RowKey` does with rows that **have** a key. The LCS over
  keyed rows is correct and is not in question.
- **Do not implement positional pairing of blank-key rows in this unit** — see
  "What this unit is not" below.
- Do not change `max_alignment_product` or the alignment cost.

## Required implementation

**The fix is to stop lying, not to align better.** Rows without a key cannot be
matched *by key*; what they must not do is disappear.

1. **A row lacking a value in every key column is unmatched, not absent.** It
   appears in `removed` (old side) or `inserted` (new side), so its cells reach
   the comparison as a whole-row change rather than vanishing. A caller then
   sees a difference where there is one — noisier than ideal, and correct.
2. **A `Warning` diagnostic naming the count**, per sheet: how many rows on each
   side had no key. Use the sheet-level location helper (`sheet_location`), as
   the other two alignment warnings do. A caller reading warnings — which is
   what we told ForskScope to do — then learns that alignment skipped rows.
3. **`confidence` must not be `Exact` when rows had no key.** This is the part
   that makes the current behaviour a false claim rather than an omission.
   Decide the value and say why in the review request; `Low` or `Medium` are
   both defensible, `Exact` is not.

## Required tests

- **ForskScope's case, as a test**: a row with a blank key column, a cell
  changed in that row, `RowKey` on that column. It must report the change.
  **Show it failing first** — on the current code it reports zero.
- The same workbook under `Positional` reports the same change (the control that
  proves the fixture is sound).
- The warning fires, with the right count, on both sides.
- `confidence` is not `Exact` when a row was keyless.
- **A sheet where every row has a key is unaffected** — no warning, same
  mapping, same diffs as before. This is the guard on Non-change scope, and the
  corpus alignment fixtures should demonstrate it byte-identically.

## Acceptance criteria

1. A change in a blank-key row is reported under `RowKey`.
2. Keyless rows appear in `removed` / `inserted`; none is silently absent.
3. A `Warning` names the per-side count; `confidence` is not `Exact`.
4. Fully-keyed sheets are unaffected — corpus goldens byte-identical, stated.
5. Every test shown failing first by a targeted change.
6. RFC-011 records the behaviour: **`RowKey` requires a key, and says what it
   does when there is not one.**
7. CHANGELOG under `### Fixed`, naming the versions affected — `RowKey` has
   behaved this way since alignment shipped; check which release that was rather
   than assuming.
8. Gates green, including `cargo check --manifest-path fuzz/Cargo.toml --bins`.

## Prohibited shortcuts

- **Do not fix this by documenting it.** ROADMAP §6 rates a missed difference
  with a crash, and "rows without a key are skipped" is not something a caller
  infers from `RowKey`'s name.
- Do not treat a blank key as the empty-string key. That silently groups every
  keyless row into one key and then reports them as duplicates — a different
  wrong answer.
- Do not suppress the warning when the count is small.

## What this unit is not

ForskScope suggested comparing keyless rows **positionally against their
neighbours**, which would be better than reporting them as removed+inserted.
**That is a quality improvement and it is not this unit.** It needs a design —
which neighbour, what happens when the counts differ between sides — and this
unit's job is to stop a wrong answer from shipping. Correctness now, quality
after, and say so in the CHANGELOG so a reader knows the noise is deliberate.

## Known risks

- **Corpus alignment fixtures may move.** They should not — their rows are fully
  keyed — but check rather than assume, and if one moves, that fixture had a
  keyless row and the movement is the defect being fixed.
- `lcs_match` takes the two key maps; threading unmatched rows through may change
  its signature. It is `pub(crate)`; no public API changes here.

## Required evidence

- The failing-first transcript for ForskScope's case
- The `Positional` control
- Corpus byte-comparison
- Which release first shipped this behaviour, established from the history
- CI run link

## Review request format

Per development policy §9.2. Additionally: state the `confidence` value you
chose and why, and confirm from the history which releases are affected — the
CHANGELOG entry needs it and my "since alignment shipped" is an assumption.
