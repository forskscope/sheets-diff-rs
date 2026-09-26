# Handoff f131-01 — The coordinate loops scan the whole map once per row

**Governing.** RFC-035 (resource safety); ROADMAP §6
**Sequence.** **Next.** Ahead of M9's remaining units.
**Release:** the next minor, with cancellable alignment.

## Purpose

An aligned comparison builds its coordinate set in O(rows × cells), and no
`Limits` field bounds it. A one-row sheet against a 40,000-row sheet takes
**37 seconds** where `Positional` takes 0.13 s.

## Background

`src/diff.rs:478` and `:487` (and the matched loop above them):

```rust
for (_, c) in old_map.keys().filter(|(row, _)| row == r) { … }
```

A **full scan of the cell map, once per row**. Measured here, old sheet one row,
3 columns:

| | `Positional` | `RowKey` | `RowSignature` |
|---|---:|---:|---:|
| 1 × 2,500 | 9 ms | 137 ms | 138 ms |
| 1 × 5,000 | 17 ms | 531 ms | 531 ms |
| 1 × 10,000 | 34 ms | 2,177 ms | 2,186 ms |
| 1 × 40,000 | 0.13 s | 37 s | — |

**Doubling the rows quadruples the time**, and a sheet may have 1,048,576.

**It is not specific to keyed alignment.** The loops are gated on a row mapping
existing, not on the mode, so `RowSignature` pays it identically — confirmed
above. Any non-`Positional` comparison is affected.

**`max_alignment_product` does not bound it.** That bounds `old_rows × new_rows`
— 1 × 40,000 = 40,000 against a default of 25,000,000. A reader would expect the
alignment bound to cap alignment cost; it caps the LCS table and nothing else,
and the threat model's Alignment paragraph describes only the table.

**Reachable in a real design.** ForskScope run `Positional`, and on a cascade
also run `RowSignature` and keep the smaller result. A one-row sheet against a
huge one is **the largest cascade `Positional` can produce** — so their trigger
fires hardest on precisely the input this is worst on. They are building a
row-asymmetry guard; this unit is what lets them delete it.

## Change scope

- `src/diff.rs` — the three loops
- `tests/`
- `docs/src/maintainers/performance.md`, `docs/src/maintainers/threat-model.md`
- `CHANGELOG.md`

## Non-change scope

- **Do not change what is compared.** Identical `WorkbookDiff` for every input:
  same coordinates, same order, same diffs. This changes *when* the work
  happens, not *what*.
- Do not change alignment, the LCS, or f130's pairing.
- Do not add or change a `Limits` field. Whether this deserves its own bound is
  a separate question — **report a view, do not act on it.**
- Do not remove the cancellation polls the previous unit added.

## Required implementation

1. **Replace each per-row full scan with a range query.** `CellMap` is a
   `BTreeMap<(u32, u32), _>`, so `map.range((r, 0)..=(r, u32::MAX))` yields that
   row's cells in O(log n + matches) instead of O(cells).
2. **All three loops** — matched (both sides), removed, inserted.
3. **Keep one cancellation poll per row.** The poll's justification changes —
   it is no longer "one poll per full scan" — so **update the comment that says
   so**, or it becomes a false statement about why the poll is there. The
   granularity is now much finer per poll, which is strictly better.
4. **`performance.md`'s phase table** is re-measured: the coordinate-set column
   is what this unit changes, and the table is currently the evidence for why
   the polls exist.
5. **The threat model's Alignment paragraph** gains the cost and says what
   bounds it after this change. If the answer is "nothing, but it is now
   linear", say that.

## Required tests

- **The asymmetric case, as a test**: a one-row sheet against a much larger one
  completes in time comparable to `Positional`. **A ratio, not a wall-clock
  figure** — e.g. aligned < some small multiple of positional on the same pair —
  so it survives a CI runner. Pick the multiple from measurement and defend it.
- **Identical results**: for every corpus scenario × every alignment mode, the
  full `WorkbookDiff` is byte-identical before and after. The previous unit ran
  76 such pairs; do the same.
- **Show it failing** by restoring one `filter` scan.
- The existing cancellation tests still pass unchanged.

## Acceptance criteria

1. All three loops use a range query; no per-row full scan remains.
2. **Results byte-identical**, shown across scenarios × modes.
3. The asymmetric test passes as a ratio and is shown failing with one scan
   restored.
4. Cancellation polls retained and their comment corrected.
5. `performance.md`'s phase table re-measured; the threat model states the cost
   and its bound.
6. Corpus byte-identical; goldens unmoved.
7. CHANGELOG `### Fixed`, naming the shape (asymmetric row counts) and that
   `max_alignment_product` never bounded it.
8. Gates green, including `cargo check --manifest-path fuzz/Cargo.toml --bins`.

## Prohibited shortcuts

- **Do not add a `Limits` field to cap it.** Making it cheap is the fix; a bound
  on a cost you can remove is a worse answer, and adding a public field is a
  minor's worth of API surface for a problem that disappears.
- Do not special-case asymmetry. The range query is right for every shape.
- Do not drop the polls because the loops are now fast — they are still O(rows)
  and a large sheet still takes time.

## Known risks

- **`range` over a tuple key needs the right bounds.** `(r, u32::MAX)`
  inclusive is the row's last possible column; check the boundary rows (column
  0, and the maximum column) rather than assuming.
- The matched loop collects both sides' columns and unions them; keep that
  behaviour exactly — a matched row's new side may carry columns the old side
  does not.
- If the re-measured phase table shows the coordinate set is *still* dominant,
  that is a finding and this fix was not the whole story. Report it.

## Required evidence

- The asymmetric measurement, before and after, at three scales
- The byte-identical comparison across scenarios × modes
- The failing-first transcript
- The re-measured phase table
- Gates with exit statuses; CI run link

## Review request format

Per development policy §9.2. Additionally: state whether the coordinate set is
still the dominant phase after the change, and give a view on whether this cost
deserves a `Limits` field of its own now that it is linear.
