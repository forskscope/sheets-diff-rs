# Handoff 04 — Delete the alignment clone

**Governing RFC.** RFC-024 (large-workbook memory)
**Roadmap.** M7 — the only build candidate the measurement justified
**Sequence.** After units 01–03, all merged. Closes M7's implementation work.

## Purpose

Remove the largest measured memory cost this crate controls: **+33% of peak**,
paid by every non-`Positional` comparison, for a copy nothing needs.

## Background

Unit 01 measured it at two scales — 32.7% of `Positional` peak at 500 rows, 33.9%
at 5,000, and ~296 bytes/row at both. Linear, reproducible, and the largest item
on the candidates table by a factor of two and a half.

`src/diff.rs:53`:

```rust
fn cell_map_to_align(cells: &CellMap) -> AlignCellMap {
    cells.iter().map(|(k, v)| (*k, v.value.clone())).collect()
}
```

Called twice at `diff.rs:384-385`, once per side. So for a non-`Positional`
comparison every `CellValue` in both sheets exists **twice**, in two separate
`BTreeMap`s.

**Alignment never needs the owned value.** Every function in `align.rs` that
takes an `AlignCellMap` was checked during scoping:

| Function | What it does with a value |
|---|---|
| `compute_row_mapping` | delegates |
| `distinct_row_count` | **`cells.keys()` only** — never touches one |
| `row_key_alignment`, `row_signature_alignment`, `header_column_alignment` | delegate |
| `extract_row_keys` (line 296) | `val.display_string()` |
| `compute_row_signatures` (line 316) | `val.display_string()` |

Two call sites, both reading a string off a value they could have borrowed. The
clone exists to serve them.

## Change scope

`src/diff.rs`, `src/align.rs`, `benches/memory.rs` (the confirming measurement),
`docs/src/maintainers/performance.md`, `rfcs/done/024-large-workbook-memory-strategy.md`,
`CHANGELOG.md`.

## Non-change scope

- **No behaviour change whatsoever.** Same diff results, same row mappings, same
  diagnostics. **The fixture corpus must not move** — if a golden moves, stop and
  report, because that means the alignment decision changed and this unit is
  wrong.
- Do not change any public API. `mod align` is private (`src/lib.rs:43`) and
  `AlignCellMap` is not exported; keep it that way.
- Do not touch the density question (RFC-024 §7). It is declined — see the
  milestone README.
- Do not restructure `CellMap` or `NormalizedCell`.

## Required implementation

1. **Have alignment borrow instead of receiving a copy.** Change
   `compute_row_mapping` and its callees to take `&CellMap`, and read
   `v.value.display_string()` at the two sites that need a value.
2. **Delete `cell_map_to_align` and the `AlignCellMap` type alias.** Both become
   dead. This unit should be net-negative in lines; if it is not, say why.
3. **`CellMap` needs `pub(crate)`.** It is currently private to `diff.rs`
   (`type CellMap = BTreeMap<(u32, u32), NormalizedCell>;` at line 35) and
   `align.rs` will need to name it. This is the one mechanical wrinkle; it was
   found during scoping so it is not a surprise.
4. **Update `align.rs`'s test helper.** `make_cells` returns an `AlignCellMap`
   and must return a `CellMap`. Its 14 call sites all pass
   `&[(u32, u32, &str)]` and should need no change — **if any of them does,
   that is worth reporting**, because it would mean a test depends on the
   intermediate type rather than on alignment behaviour.

## Required tests

**The corpus is the test**, and it is a strong one: `alignment_row_signature`
and `alignment_header_column` exercise both non-`Positional` modes, and
`row_insertion_cascade` exercises alignment at scale. If alignment behaviour
changed, they move.

Beyond that:

1. **Confirm the corpus is byte-identical**, and say so explicitly rather than
   relying on the suite passing.
2. **Re-run unit 01's align-clone isolation** (`benches/memory.rs`, Q2) at both
   row counts and report the new numbers. The measured delta should collapse
   toward zero — **that is this unit's proof of effect**, and without it the
   claim of 33% is asserted rather than demonstrated.
3. **Confirm `Positional` peak is unchanged.** It never paid this cost, so it
   should not move. If it does, something else changed.

## Acceptance criteria

1. Alignment takes `&CellMap`; no intermediate map is built.
2. `cell_map_to_align` and `AlignCellMap` are gone.
3. Fixture corpus byte-identical, stated explicitly.
4. Unit 01's Q2 isolation re-run at both row counts, new numbers reported, and
   the delta collapses toward zero.
5. `Positional` peak unchanged.
6. No public API change; `mod align` still private.
7. `performance.md`'s Q2 section records the before and after — **do not
   overwrite the pre-fix numbers**; they are the record of what was measured.
8. The candidates table marks this **Done**, matching how unit 03 recorded the
   cancellation row.
9. RFC-024's Status updated for this item only, its other clauses checked first.
10. CHANGELOG under `### Changed` — peak memory for non-`Positional`
    comparisons drops materially; no observable result differs. Gates green,
    full matrix, including the scoped stdout gate and MSRV doctests.

## Prohibited shortcuts

- **Do not keep `AlignCellMap` "just in case".** An unused type alias in a
  private module is the kind of thing that survives for four releases and then
  needs a doc comment explaining why it is unreachable. This project has been
  cleaning those up since M4.
- Do not make `align` public to solve an import problem. `pub(crate)` on
  `CellMap` is the answer.
- Do not accept "tests pass" as proof the corpus is unchanged. Check the bytes.
- Do not skip the re-measurement because the change is obviously an improvement.
  Obvious is what M7 exists to distrust — and a measurement that does *not*
  collapse would mean the 33% was attributed to the wrong thing.

## Known risks

- **Borrow conflicts are the thing to watch, and scoping suggests there are
  none**: `RowMapping` contains only `u32` data (`matched: BTreeMap<u32,u32>`,
  `removed`/`inserted`: `Vec<u32>`), so it borrows nothing and the immutable
  borrow ends at the `compute_row_mapping` call, before the compare loop reads
  the same maps. If you hit a borrow error anyway, that is a finding — report
  what it reveals rather than working around it with a clone.
- The two `display_string()` sites allocate a `String` each. That is unchanged
  by this unit and is *not* the 33% — do not conflate them, and do not
  opportunistically optimise them here.

## Required evidence

- The diff (expected net-negative)
- Corpus byte-comparison
- Unit 01's Q2 isolation, before and after, both row counts
- `Positional` peak, before and after
- CI run link

## Review request format

Per development policy §9.2, plus the before/after isolation numbers — this
unit's claim is a measured quantity, so the measurement is the deliverable
alongside the code.
