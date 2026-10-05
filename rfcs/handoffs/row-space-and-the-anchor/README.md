# Row space, and the anchor that does not identify

**Added 2026-10-05.** **Scoped by:** the architect, from
`.git-exclude/decisions/006-the-row-space-field.md`.
**Target releases:** unit 00 is documentation-only and ships in **3.4.0**, decided by the owner
2026-10-06, alongside `the-row-that-vanishes/01` — separate reviews, one release, and that unit goes
first. (It was 3.3.1 until that unit's semver label was corrected: it adds a `DiagnosticKind`
variant, which is a minor.) Units 01 and 02 then move to **3.5.0**; the owner holds that schedule.
**Revised 2026-10-05**, after the consumer's reply — decision 006 §8. Unit 01's shape changed and a
third unit was added. If you are holding an earlier copy of unit 01, re-read it.

## Why this exists

A consumer asked for one field. Designing it found that **D-03 was fixed in the engine and dropped
at the boundary.**

D-03's recorded fix direction (`rfcs/handoffs/035-…/05-integrity-defects.md`):

> *"the coordinate set needs to carry which side a row number came from, rather than relying on
> numeric identity across two coordinate spaces."*

`CoordKey` carries it as far as `src/diff.rs:585`, where the `CellDiff` is built — and no further.
Every layer below still relies on numeric identity across the two row spaces.

## The reproduction units 01 and 02 use

Three rows per side, `AlignmentMode::RowKey { columns: vec![1] }`:

```
old:  k1/a    k2/b     kdel/x
new:  k1/a    knew/z   k2/bb
```

k2 matches old-row 2 ↔ new-row 3 with a changed value; `kdel` is removed (old row 3); `knew` is
inserted (new row 2). So old-row 2 and new-row 2 are both live and unrelated. Measured on 3.3.0:

```
cell_diffs:                          view anchors:
  A2  row=2 col=1  Added               { sheet_index: 0, row: 2, col: 1 }
  B2  row=2 col=2  Modified            { sheet_index: 0, row: 2, col: 2 }
  B2  row=2 col=2  Added               { sheet_index: 0, row: 2, col: 2 }   <-- duplicate
  A3  row=3 col=1  Removed             { sheet_index: 0, row: 3, col: 1 }
  B3  row=3 col=2  Removed             { sheet_index: 0, row: 3, col: 2 }
```

`B2 Modified` is k2's changed value in the **old** row space. `B2 Added` is `knew`'s cell in the
**new** row space. Nothing in the public API separates them.

**Build this as a shared helper once** — `tests/integration.rs` already has the in-memory
`rust_xlsxwriter` pattern at `row_key_alignment_reduces_cascade` (~`:1140`). Units 01 and 02 both
assert against it. Do not make it a corpus fixture; see *Why the corpus cannot carry this* below.
Unit 01 extends it so `k1`'s value changes too, which gives a paired row numbered 1 on both sides —
the case that distinguishes a real pairing from a positional comparison.

## The three units

| Unit | What | Semver |
|---|---|---|
| **00** | two doc comments that are false in shipped 3.3.0, one of them harmful | **patch — prose only** |
| **01** | `row_placement` on `CellDiff` — the row numbers a cell change does not carry | minor |
| **02** | the view layer's navigation hang, and `ChangeAnchor`'s false promise | minor |

**Order: 00, then 01, then 02.**

**00 first and alone.** It is prose, its `cargo public-api` diff must be empty, and it can ship
without waiting for anything. The reason it goes first is `src/model.rs:526`, which tells consumers
to *"collapse to one row per address"* — an instruction that merges two distinct changes under any
aligned mode. A consumer has told us they read that sentence and avoided it *"by luck rather than
judgement"*. Every consumer reading our docs before 3.4.0 gets the wrong instruction, for no gain.

**Then 01.** A consumer is blocked on it and we are nine days late on a written promise.

**Then 02.** A live hang in shipped code — though, having checked, it reaches no consumer we know
of; see below.

01 and 02 are independent: 01 touches the model and the engine's single construction site, 02
touches `src/output/view.rs`. Unit 00 touches the doc comments both of them will then have to keep
true.

## Why the corpus cannot carry this

`generated_fixtures_match_golden` compares with `compare_bytes(&old, &new)` — **default options**,
and `AlignmentMode::Positional` is the default. All 25 goldens will therefore show exactly one
`row_space` value, `NumberedInBothSheets`, including `row_insertion_cascade`, whose whole point is
the *positional* cascade.

So the goldens will move (every cell diff gains a key) while proving nothing about the two variants
that matter. **The dedicated non-default-alignment assertions are the actual coverage here, not the
corpus.** This is the same shape as M9's finding that the corpus cannot reach its own warnings —
state it in the review request so it lands in RFC-036's matrix rather than being rediscovered.

## Who the view defect reaches — checked, not assumed

The consumer looked: `DiffView`, `ChangeAnchor`, `next_after`, `previous_before` and `output::view`
*"appear nowhere in our codebase"*. They build their own navigation on their own model. So the hang
reaches **no consumer we currently know of**, and they asked us not to weigh unit 02's deprecation
trade on their behalf.

That does not make unit 02 optional — the defect is real and reachable through a documented API —
but it is why the *documentation* half of it was pulled forward into unit 00 and the rest was not.
Their point, which I accepted: *"The deprecation can take its window; the sentence should not."*

## What is deferred, deliberately

- **`ChangeAnchor` gaining the row space.** It is a closed, field-public struct, so that is a major.
  Unit 02 adds a correct route alongside and deprecates the broken one; the repair itself goes to v4.
- **`#[non_exhaustive]` on the `view.rs` types.** There is none anywhere in that file, which is
  precisely why unit 02's real fix is a major. v4.
- **A per-row alignment confidence.** Not asked for, and it does not belong inside a row's
  placement. `AlignmentSummary.confidence` is per-sheet and stays that way.

**No longer deferred:** a matched row's new-side row number. I deferred it on 2026-10-05 as
speculative; the consumer replied the same day that the field is unusable without it, because they
render two documents, one per side, and the new-side pane would print a row number that is not
where that row lives in the new file. It is in unit 01's shape. Decision 006 §8.1 has the reasoning
and the correction.

## Standing rules

Every unit: rule 002 (one scratch target dir per sweep, deleted after), rule 003 (sweep both
manifests — `./Cargo.toml` and `./fuzz/Cargo.toml`, the latter only via
`cargo check --manifest-path fuzz/Cargo.toml --bins`). Do not commit; the review comes first.
