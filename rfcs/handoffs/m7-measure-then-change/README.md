# Handoffs — M7: measure, then change

**Open 2026-08-17.** This directory was written before the milestone was
proposed, because M7's discipline is that **the rest of its scope cannot
honestly be written until something is measured** — so the measurement unit had
to exist before the milestone could.

Governed by RFC-024 (large-workbook memory), RFC-012 (progress, cancellation and
resource bounds), RFC-027 (benchmark and performance governance). All in
`done/`, all carrying named gaps.

## Why this milestone is shaped differently

Every previous milestone knew what it was fixing before it started. M4 knew the
eight statements that were false. M5 knew the three rules nothing checked. M6
knew the two MUSTs.

M7 does not, and pretending otherwise is how a performance milestone turns into
a refactor nobody asked for. Three of its four candidate items have the same
property: **we know a cost exists and not what it is.**

- `compare_bytes` copies its input. The threat model says this doubles peak
  memory. **Nobody has measured it.**
- `cell_map_to_align` clones every `CellValue` in both sheets. That is a real
  allocation and an unknown fraction of the total.
- Cancellation is polled once per sheet pair. RFC-024's status calls this a gap
  against acceptance criteria; RFC-012's own goal says *"cancellation checks at
  major pipeline stages"*, which a sheet pair arguably is. **The disagreement is
  unresolvable without knowing how long a caller actually waits.**

This project has guessed at a cause and been wrong twice — the synthetic-fixture
misdiagnosis, and the "G needs v3" claim. Unit 01 exists so M7 does not make it
a third time.

## Queue

| | Unit | Governing | Status |
|---|---|---|---|
| 01 | [Measure](./01-measure.md) | RFC-024, RFC-012 | ✅ merged — reordered the milestone |
| 02 | [The v1.2-vs-v2 comparison](./02-v1-2-comparison.md) | RFC-027 | ✅ merged — RFC-027 fully implemented |
| 03 | [Cancellation that fires](./03-cancellation-that-fires.md) | RFC-012, RFC-024 | ✅ merged `db88706` |
| 04 | [Delete the alignment clone](./04-delete-the-alignment-clone.md) | RFC-024 | **Ready** — the only candidate the measurement justified |

Unit 02 moved here from M6 — it is measurement, not documentation. It is
independent of unit 01 and gates nothing, but it is **not the easy one**: v1.2
resolved `calamine` 0.35.0 where we run 0.36.1, it compared cells as strings
where v2 normalises and aligns, and it shipped no benchmarks at all. A naive
comparison would produce a number that is partly upstream's and partly the cost
of capability v1.2 did not have. The unit's difficulty is method, not
mechanics.

**Unit 01 has reported, and it reordered the milestone.** Three of its four
answers differed from what reading the code suggested, and one was not a
measurement at all but a defect: **cancellation is never observed on a
single-sheet workbook** — not coarse, absent. `check_cancel` fires once per
sheet pair, before that sheet's work, so on the ordinary one-sheet case a
caller who cancels is never seen and the comparison returns `Ok`.

That is a feature that does not work, and it now leads the milestone as unit 03,
ahead of every memory candidate. Those are optimisations.

**Order: 01 → 03 → 02 → 04+.** *(Clarified 2026-08-17 after the dev team asked;
the text above ordered 03 ahead of units 04+ and said nothing about 02, which
was a real ambiguity.)* Unit 02 is still independent in the sense that nothing
blocks it — but **unit 03 modifies the code unit 02 measures.** Unit 03 adds a
polling checkpoint inside both per-sheet loops (`src/diff.rs:472` and `:595`),
which are exactly the hot paths unit 02 benchmarks. Running 02 first would
publish a v1.2-vs-v2 comparison against a version of v2 that stops existing when
03 lands.

The overhead will likely be well under a percent, and that is not the point:
this milestone's discipline is that a published number stays attributable to
something still true. Measure the engine we ship. Unit 02's report must also
name the commit it measured, since "v2" is no longer one thing across this
milestone.

**The memory candidates were decided on 2026-08-17, on the measurements rather
than on intuition: one accepted, two declined.**

- **Accepted — the alignment clone (+33% of peak).** Unit 04. Alignment only
  ever calls `display_string()` on the values it is handed, so the copy is
  *deletable* rather than reducible, and `mod align` is private so nothing
  public moves.
- **Declined — RFC-024 §7's density choice (+12.4%).** The smallest structural
  change available for the largest increase in engine complexity: a density
  heuristic and two code paths through the loop where every silent-wrong-answer
  defect this project has fixed lived.
- **Declined — `compare_bytes`'s copy (+2.6–4.8%).** The one we had called a
  doubling. A cheap route does exist — the internals already take an owned
  `Vec<u8>`, and only the public `AsRef<[u8]>` bound forces the copy — but it
  must be additive (`&Vec<u8>` does not satisfy `Into<Vec<u8>>`), recovers only
  the input's ~2.5% share, and costs permanent public API on a crate whose
  non-goals page already explains a model larger than its engine.

**Declined, not deferred**, and recorded with their numbers in
[`performance.md`](../../../docs/src/maintainers/performance.md) and RFC-024's
Status. "Deferred" is what these were for four releases and carries no
information; "declined, and here is the measurement" tells the next person why.

Two of three declined is the milestone succeeding. M7 existed to find out which
were worth building, and it found that the item we were most confident about is
the smallest, that the tidiest-sounding one buys least for most disruption, and
that the real work was somewhere nobody had listed.

## The candidate items, and what would settle each

Measured by unit 01. Full method and figures:
[`docs/src/maintainers/performance.md`](../../../docs/src/maintainers/performance.md).

| Candidate | Measured | Verdict |
|---|---|---|
| Finer cancellation polling | Never fires on one sheet; ~567 ms on two | **Unit 03.** Not an optimisation — a broken feature |
| Reduce `cell_map_to_align`'s clone | **+33% of peak**, ~296 B/row at two scales | Largest real win. Non-`Positional` modes only |
| RFC-024 §7's density choice | +12.4% per populated cell, dense vs sparse | Real, modest. Not urgent |
| Remove `compare_bytes`'s copy | **+2.6–4.8%**, not the doubling we claimed | Small. The threat model was wrong and is corrected |
| Not holding both `CellMap`s | Not isolable by external measurement | No actionable number without instrumenting `src/` |
| Shared display address (G) | N/A | Design question, additive on `#[non_exhaustive]` types |

## Standing constraints

- **Unit 01 changes no library code.** It measures. If it finds a defect,
  report it — do not fix it.
- **No optimisation before the report.** A change that looks obviously good is
  exactly what measurement is for; this project has been wrong about obvious
  before.
- **Report what the measurement does not capture.** Every method has a boundary
  — allocator-tracked bytes are not RSS, a single process is not a GUI under
  load. The boundary is part of the result.
- **The fixture corpus must not move.**
- Gates as always: fmt, clippy `-D warnings`, the scoped stdout gate, `deny`,
  MSRV 1.88, the full matrix.
