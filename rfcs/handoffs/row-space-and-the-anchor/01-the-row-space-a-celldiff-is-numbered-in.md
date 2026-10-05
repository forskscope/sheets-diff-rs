# Handoff 01 — The row space a `CellDiff`'s address is numbered in

**Unit:** row-space-and-the-anchor 01. **Added 2026-10-05.**
**Scoped by:** the architect, from `.git-exclude/decisions/006-the-row-space-field.md` §4.
**Semver:** minor. `CellDiff` is `#[non_exhaustive]`, so a field is additive.
**Read `README.md` in this directory first** — it carries the reproduction and why the corpus cannot
cover this.

## Purpose

`CellDiff` carries one `address` and no indication of which sheet's row numbering that address uses.
Add it. The engine already knows: `CoordKey`'s three variants are exactly the distinction, and the
match at `src/diff.rs:534` discriminates them four lines above the single `CellDiff` construction
site at `:585`, where it is discarded.

## Background you need

Two things make this more than a convenience field.

**1. Two `CellDiff`s can share one address.** See the README's reproduction: `B2 Modified` (old row
space) and `B2 Added` (new row space) in one sheet. Without the field, a consumer holding one
`(row, col)` per change cannot represent both.

**2. Our own public doc instructs consumers to merge them.** `src/model.rs:526`, in bold:

> *"**One `CellDiff` per logical address.** … Consumers migrating from a per-facet model should
> **collapse to one row per address** rather than preserve the split."*

A consumer who follows that sentence merges an inserted row's cell with a matched row's cell.
**The field is not an addition to the identity — it is the missing part of an identity we already
claim is unique.** That paragraph has to change in this unit, not later.

## The shape — settled, do not redesign it

```rust
/// Which sheet's row numbering [`CellDiff::address`] is expressed in.
///
/// This says where the **row number** comes from — not which sheet the cell is in. A matched row
/// exists in both sheets and is numbered in the old one's.
pub enum RowSpace {
    /// The row number is the old sheet's. **Matched and removed rows both land here.**
    NumberedInOldSheet,
    /// The row number is the new sheet's. **Only inserted rows land here** — they have no old-side
    /// counterpart to be numbered against.
    NumberedInNewSheet,
    /// No alignment ran: both sheets share one row-number space and the number is valid in either.
    NumberedInBothSheets,
}
```

…and `pub row_space: RowSpace` on `CellDiff`.

Three things about it are decisions, not preferences, and the review will check them:

- **The variant names are predicates about the number, not the cell.** We promised a consumer in
  writing that we would *"name it so the framing is unavoidable"*, because `Old`/`New` reads as
  "which file the cell is in". If the names feel long, that is the point. Do not shorten them.
- **`RowSpace` is exhaustive — no `#[non_exhaustive]`.** RFC-031 §6's criterion is "public enums
  likely to grow"; this domain is closed. `#[non_exhaustive]` would force consumers to write `_ =>`,
  which silently absorbs a future variant and places changes in the wrong row space — D-03's failure
  class in a consumer's code. A compile error is the better failure here. `AlignmentMode` is the
  existing precedent for an exhaustive public enum.
- **It goes on `CellDiff`, not on `CellAddress`.** `CellAddress` is also used by
  `DiagnosticLocation.address`, where there may be no row space, and its public
  `CellAddress::new(row, col)` has none to supply.

If you think any of the three is wrong, **stop and say so before implementing** — a disagreement
here is cheap now and expensive after 25 goldens move.

## Change scope

- `src/model.rs` — the enum, the field, and the "one `CellDiff` per logical address" paragraph.
- `src/diff.rs` — carry the discriminant from the existing `match *key` at ~`:534` into the
  construction at ~`:585`. This should be a few lines; if it is not, report why before continuing.
- `tests/integration.rs` — the shared reproduction helper and the dedicated assertions.
- `tests/fixtures/generated/*/expected.json` — all 25, blessed.
- `docs/src/semantics.md` — row-space semantics beside the alignment section.
- `rfcs/done/033-public-model-lexicon.md` §5 — it pins `CellDiff`'s exact shape.
- `docs/src/migration/` — a note for consumers who followed the "collapse to one row per address"
  instruction.

## Non-change scope

- **`src/output/view.rs` is unit 02's.** Do not touch it here, even though it is obviously affected.
- Do not expose `compute_row_mapping` or any part of `mod align`.
- Do not add a matched row's new-side row number. Deferred by decision; see the README.
- Do not change how alignment works. This unit projects an existing distinction and computes nothing
  new.

## Required implementation

**1. The enum and the field**, exactly as above, with those doc comments.

**2. Feed it from `CoordKey`.** The existing match already produces
`(row, col, old_lookup, new_lookup)` per key; add the row space to that tuple. Mapping is
one-to-one: `Old` → `NumberedInOldSheet`, `InsertedNew` → `NumberedInNewSheet`, `Positional` →
`NumberedInBothSheets`. **Do not re-derive it from `old_lookup`/`new_lookup` being `Some`/`None`** —
that is inference from a side effect, and it is how this information got lost in the first place.

**3. Fix `src/model.rs:526`.** The paragraph currently claims uniqueness per address and tells
consumers to collapse. Either define a logical address as *(row space, row, col)* and keep the
uniqueness claim true, or drop the claim. State which you chose and why. The sentence about
`output::view::CellChangeRow` following "the same rule" is **false today** — unit 02 owns the fix,
but this unit must not leave the sentence asserting something unit 02 has not yet delivered.

**4. Bless the goldens, and read the diff.** `tests/fixtures/corpus/README.md` is explicit that
blessing without reading is the failure mode. Expect **all 25** to change by exactly one added key
per cell diff, with the value `NumberedInBothSheets` everywhere, because the golden test uses
default options. **If any golden shows a different value, something is wrong** — the golden path
cannot reach the other two variants. Say in the review that you confirmed this, with the per-file
shape of the diff.

**5. Records.** RFC-033 §5, `docs/src/semantics.md`, the migration note.

## Required tests

All in `tests/integration.rs`, under non-default alignment, because the corpus cannot reach this:

1. **Two `CellDiff`s at one address, distinguished.** On the README's reproduction: two entries with
   `address.a1 == "B2"`, one `NumberedInOldSheet` + `Modified`, one `NumberedInNewSheet` + `Added`.
   **This is the test that proves the field carries information**; the golden does not.
2. **Removed rows are numbered in the old space**, not a fourth category — `A3`/`B3` above are
   `NumberedInOldSheet`. This is the one a reader is most likely to get wrong, and the doc comment
   calls it out for that reason.
3. **Default options give `NumberedInBothSheets`** for every cell diff.
4. **`RowSignature` alignment** also produces old/new spaces — `RowKey` is not the only non-default
   mode, and a fix that only works for one would pass tests 1–3.
5. **Failing-first for each of 1, 2 and 4**, by short-circuiting the specific mapping arm — one edit
   at a time, `cmp`-verified restore between each, never two live at once. A test that passes
   against a mutation that should break it is the thing we keep finding.

## Acceptance criteria

1. The field and enum land with the agreed names, exhaustive, on `CellDiff`.
2. Fed from `CoordKey`'s discriminant, not inferred from `Option`s.
3. All five tests, each with its failing-first demonstration.
4. All 25 goldens blessed, the diff read, and the uniform `NumberedInBothSheets` result confirmed
   and reported.
5. `src/model.rs:526` no longer tells consumers to do the wrong thing.
6. RFC-033 §5, `docs/src/semantics.md` and a migration note updated.
7. Gates green, rule 003 swept, one scratch dir, deleted. Nothing committed.
8. `cargo public-api` diff shows the addition and **nothing else** — run it in a detached worktree,
   not with a stash; the commit-range form does an in-place checkout and fails on a dirty tree.

## Prohibited shortcuts

- Do not shorten the variant names. We promised the long framing to a consumer in writing.
- Do not add `#[non_exhaustive]` to `RowSpace` "to be safe". It is the less safe choice here and the
  decision record says why.
- Do not infer the row space from `old_lookup`/`new_lookup`.
- Do not bless goldens before reading the diff.
- Do not quietly fix `view.rs` because you noticed it too. Report it to unit 02's scope.

## Known risks

**1. The JSON output changes for every consumer.** `CellDiff` derives `Serialize`. We implement no
`Deserialize`, so nothing round-trips and no key is lost — but the new key is consumer-visible and
belongs in the release notes. Note it in the review request so it reaches them.

**2. RFC-031 §8 says *adding optional fields* is minor-compatible, and `row_space` is always
present.** I read that as covered — there is no `Deserialize` and no field is removed or renamed —
but §8's wording does not literally address an always-present addition. **Flag it; do not amend §8
yourself.** That is mine.

**3. Hardcoded corpus counts.** M9 unit 05 had to bump several. This unit adds no scenario, so they
should not move. If one does, stop and report — it means something other than the key changed.

**4. The `view.rs` sentence in `src/model.rs:526`.** See *Required implementation* 3. Easy to leave
asserting a rule unit 02 has not yet made true.

## Required evidence

Under `.git-exclude/review-request/row-space-01-the-row-space/evidence/`:

1. The `CoordKey` → `RowSpace` threading as a diff, showing it comes from the discriminant.
2. Each of the five tests, with its failing-first mutation and the `cmp`-verified restore.
3. The golden diff: that all 25 moved, by one key, uniformly `NumberedInBothSheets`.
4. `cargo public-api` output from a detached worktree.
5. Gate sweep across both manifests; the scratch dir created and deleted.

## Review request format

`.git-exclude/review-request/row-space-01-the-row-space/README.md`, with:

- Whether you agreed with the three settled decisions, and if not, what you would have done.
- What you chose for `src/model.rs:526`, and the exact new wording.
- Confirmation that every golden shows `NumberedInBothSheets`, and that you understand why.
- The RFC-031 §8 question, stated for me to decide.
- Anything in `view.rs` you noticed and left for unit 02.
