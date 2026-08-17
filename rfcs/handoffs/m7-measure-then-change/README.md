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
| 01 | [Measure](./01-measure.md) | RFC-024, RFC-012 | **Ready** — gates everything else |
| 02 | [The v1.2-vs-v2 comparison](./02-v1-2-comparison.md) | RFC-027 | **Ready** — separable, gates nothing |
| 03 | [Cancellation that fires](./03-cancellation-that-fires.md) | RFC-012, RFC-024 | **Ready** — scoped from unit 01's report |
| 04+ | Memory candidates, scope set by unit 01's report | RFC-024 | Not yet written |

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

The memory candidates remain unwritten pending a decision on which are worth
doing — see the table below, now populated with measured sizes.

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
