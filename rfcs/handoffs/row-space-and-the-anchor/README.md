# Row space, and the anchor that does not identify

**Added 2026-10-05.** **Scoped by:** the architect, from
`.git-exclude/decisions/006-the-row-space-field.md`.
**Target release:** 3.4.0 (both units are minor; the owner holds the schedule).

## Why this exists

A consumer asked for one field. Designing it found that **D-03 was fixed in the engine and dropped
at the boundary.**

D-03's recorded fix direction (`rfcs/handoffs/035-…/05-integrity-defects.md`):

> *"the coordinate set needs to carry which side a row number came from, rather than relying on
> numeric identity across two coordinate spaces."*

`CoordKey` carries it as far as `src/diff.rs:585`, where the `CellDiff` is built — and no further.
Every layer below still relies on numeric identity across the two row spaces.

## The reproduction both units use

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
`rust_xlsxwriter` pattern at `row_key_alignment_reduces_cascade` (~`:1140`). Both units assert
against it. Do not make it a corpus fixture; see *Why the corpus cannot carry this* below.

## The two units

| Unit | What | Semver |
|---|---|---|
| **01** | `row_space` on `CellDiff` — the missing part of a cell change's identity | minor |
| **02** | the view layer's navigation hang, and `ChangeAnchor`'s false promise | minor |

They are independent: 01 touches the model and the engine's single construction site, 02 touches
`src/output/view.rs`. **01 first** — a consumer is blocked on it and we are nine days late on a
written promise. Take 02 immediately after; it is a live hang in shipped code.

## Why the corpus cannot carry this

`generated_fixtures_match_golden` compares with `compare_bytes(&old, &new)` — **default options**,
and `AlignmentMode::Positional` is the default. All 25 goldens will therefore show exactly one
`row_space` value, `NumberedInBothSheets`, including `row_insertion_cascade`, whose whole point is
the *positional* cascade.

So the goldens will move (every cell diff gains a key) while proving nothing about the two variants
that matter. **The dedicated non-default-alignment assertions are the actual coverage here, not the
corpus.** This is the same shape as M9's finding that the corpus cannot reach its own warnings —
state it in the review request so it lands in RFC-036's matrix rather than being rediscovered.

## What is deferred, deliberately

- **`ChangeAnchor` gaining the row space.** It is a closed, field-public struct, so that is a major.
  Unit 02 adds a correct route alongside and deprecates the broken one; the repair itself goes to v4.
- **`#[non_exhaustive]` on the `view.rs` types.** There is none anywhere in that file, which is
  precisely why unit 02's real fix is a major. v4.
- **A matched row's new-side row number.** Still unavailable, by decision, not oversight. Nobody has
  asked for it. Do not add it.

## Standing rules

Both units: rule 002 (one scratch target dir per sweep, deleted after), rule 003 (sweep both
manifests — `./Cargo.toml` and `./fuzz/Cargo.toml`, the latter only via
`cargo check --manifest-path fuzz/Cargo.toml --bins`). Do not commit; the review comes first.
